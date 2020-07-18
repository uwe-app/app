use std::sync::Arc;
use std::path::PathBuf;

use crossbeam::channel;
use log::error;

use book::compiler::BookCompiler;
use config::Page;

use crate::{Error, Result};
use crate::context::BuildContext;
use crate::hook;
use crate::run;
use crate::parser::Parser;
use crate::resource;

pub struct Compiler<'a> {
    pub context: &'a BuildContext,
    pub book: BookCompiler,
    parser: Parser<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(context: &'a BuildContext) -> Result<Self> {

        let book = BookCompiler::new(
            context.options.source.clone(),
            context.options.target.clone(),
            context.options.settings.is_release(),
        );

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(&context)?;

        Ok(Self {
            context,
            book,
            parser,
        })
    }

    // Verify the paths are within the site source
    pub fn verify(&self, paths: &Vec<PathBuf>) -> Result<()> {
        for p in paths {
            if !p.starts_with(&self.context.options.source) {
                return Err(Error::OutsideSourceTree(p.clone()));
            }
        }
        Ok(())
    }

    // Try to find page data for a file from the collation
    fn get_page(&self, file: &PathBuf) -> Option<&Page> {
        if let Some(ref opt) = self.context.collation.all.get(&Arc::new(file.clone())) {
            opt.as_ref()
        } else {
            None
        }
    }

    // Build a single file
    pub async fn one(&self, file: &PathBuf) -> Result<()> {
        if let Some(page) = self.get_page(file) {
            let mut data = page.clone();
            let render = data.render.is_some() && data.render.unwrap();
            if !render {
                return run::copy(self.context, file).await
            }
            run::parse(self.context, &self.parser, file, &mut data).await?;
        } else {
            run::copy(self.context, file).await?;
        }
        Ok(())
    }

    pub async fn build(&self, target: &PathBuf) -> Result<()> {
        let parallel = self.context.options.settings.is_parallel();

        // TODO: support allowing this in the settings
        let fail_fast = false;

        let all = self.context.collation.other.iter()
            .chain(self.context.collation.assets.iter())
            .chain(self.context.collation.pages.iter())
            .filter(|p| p.starts_with(target));

        if parallel {
            let (tx, rx) = channel::unbounded();

            rayon::scope(|s| {
                for p in all {
                    let tx = tx.clone();
                    s.spawn(move |_t| {
                        // NOTE: we pay a price for creating another runtime
                        // NOTE: inside the rayon thread but it gives us a 
                        // NOTE: consistent futures based API
                        let mut rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async move {
                            let res = self.one(p).await;
                            if fail_fast && res.is_err() {
                                error!("{}", res.err().unwrap());
                                panic!("Build failed");
                            } else {
                                tx.send(res).unwrap();
                            }
                        });

                    })
                }
            });

            drop(tx);
            let errs: Vec<Error> = rx.iter()
                .filter(|r| r.is_err())
                .map(|r| r.err().unwrap())
                .collect::<Vec<_>>();

            if errs.is_empty() {
                Ok(())            
            } else {
                Err(Error::Multi { errs })
            }
        } else {
            for p in all {
                self.one(p).await?;
            }

            Ok(())
        }
    }

    // Build all target paths
    pub async fn all(&self, targets: Vec<PathBuf>) -> Result<()> {
        let livereload = crate::context::livereload().read().unwrap();

        resource::link(&self.context)?;

        if let Some(hooks) = &self.context.config.hook {
            hook::run(
                self.context,
                hook::collect(
                    hooks.clone(),
                    hook::Phase::Before,
                    &self.context.options.settings.name),
            )?;
        }

        for p in targets {
            if p.is_file() {
                self.one(&p).await?;
            } else {
                self.build(&p).await?;
            }
        }

        // Now compile the books
        if let Some(ref _book) = self.context.config.book {
            self.book
                .all(&self.context.config, livereload.clone())?;
        }

        if let Some(hooks) = &self.context.config.hook {
            hook::run(
                self.context,
                hook::collect(
                    hooks.clone(),
                    hook::Phase::After,
                    &self.context.options.settings.name),
            )?;
        }

        Ok(())
    }

}
