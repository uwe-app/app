use std::sync::Arc;
use std::path::PathBuf;

use crossbeam::channel;
use log::{debug, error};

use book::compiler::BookCompiler;
use config::Page;

use crate::{Error, Result};
use crate::context::BuildContext;
use crate::hook;
use crate::run::{self, ParseData};
use crate::parser::Parser;
use crate::resource;

pub struct Compiler<'a> {
    pub context: &'a BuildContext,
    pub book: BookCompiler,
}

impl<'a> Compiler<'a> {
    pub fn new(context: &'a BuildContext) -> Self {
        let book = BookCompiler::new(
            context.options.source.clone(),
            context.options.target.clone(),
            context.options.settings.is_release(),
        );

        Self {
            context,
            book,
        }
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
        self.context.collation.pages.get(&Arc::new(file.clone()))
    }

    // Build a single file
    pub async fn one(&self, parser: &Parser<'_>, file: &PathBuf) -> Result<Option<ParseData>> {

        if let Some(page) = self.get_page(file) {

            let render = page.render.is_some() && page.render.unwrap();
            if !render {
                let file_ctx = page.file.as_ref().unwrap();
                return run::copy(file, &file_ctx.target).await
            }

            run::parse(self.context, parser, page.get_template(), &page).await
        } else {
            let target = self.context.collation.targets.get(file).unwrap();
            run::copy(file, target).await
        }
    }

    pub async fn build(&self, parser: &Parser<'_>, target: &PathBuf) -> Result<Vec<ParseData>> {
        let parallel = self.context.options.settings.is_parallel();

        // Filtering using the starts_with() below allows command line paths
        // to filter the files that get compiled. However if we apply this filtering
        // logic when the target is the main input source directory we will also
        // ignore synthetic asset files outside of the source directory such as
        // those copied over for the search runtime. This is a workaround for now
        // but really we should rewrite the path filtering logic.
        let filter_active = *target != self.context.options.source;

        // TODO: support allowing this in the settings
        let fail_fast = true;

        let all = self.context.collation.targets
            .iter()
            .filter(|(p, _)| {
                if !filter_active { return true }
                p.starts_with(target)
            })
            .filter(|(p, target)| {
                if let Some(ref manifest) = self.context.collation.manifest {
                    let file = p.to_path_buf();
                    if manifest.exists(&file) && !manifest.is_dirty(&file, &target, false) {
                        debug!("[NOOP] {}", file.display());
                        return false
                    }
                }
                true
            })
            .map(|(p, _)| p);

        let mut data: Vec<ParseData> = Vec::new();

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
                            let res = self.one(parser, p).await;
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

            let mut errs: Vec<Error> = Vec::new();
            rx.iter().for_each(|r| {
                if r.is_err() {
                    errs.push(r.err().unwrap());
                } else {
                    let res = r.unwrap();
                    if let Some(parse_data) = res {
                        data.push(parse_data);
                    }
                }
            });

            if !errs.is_empty() {
                return Err(Error::Multi { errs })
            }
        } else {
            for p in all {
                if let Some(parse_data) = self.one(parser, p).await? {
                    data.push(parse_data);
                }
            }
        }

        Ok(data)
    }

    // Build all target paths
    pub async fn all(&self, parser: &Parser<'_>, targets: Vec<PathBuf>) -> Result<Vec<ParseData>> {

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

        let mut data: Vec<ParseData> = Vec::new();

        for p in targets {
            if p.is_file() {
                if let Some(parse_data) = self.one(parser, &p).await? {
                    data.push(parse_data);
                }
            } else {
                data.append(&mut self.build(parser, &p).await?);
            }
        }

        // Now compile the books
        if let Some(ref _book) = self.context.config.book {
            let livereload = crate::context::livereload().read().unwrap();
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

        Ok(data)
    }

}
