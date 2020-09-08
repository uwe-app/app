use std::path::PathBuf;
use std::sync::Arc;

use crossbeam::channel;
use log::error;

use book::compiler::BookCompiler;

use collator::{Collate};

use crate::context::{BuildContext, CompilerOutput};
use crate::hook;
use crate::parser::Parser;
use crate::run;
use crate::{Error, Result};

#[derive(Debug)]
pub struct Compiler {
    pub context: Arc<BuildContext>,
    pub book: BookCompiler,
}

impl Compiler {
    pub fn new(context: Arc<BuildContext>) -> Self {
        let book = BookCompiler::new(
            context.options.source.clone(),
            context.options.base.clone(),
            context.options.settings.is_release(),
        );

        Self { context, book }
    }

    pub async fn build(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        target: &PathBuf,
        output: &mut CompilerOutput,
    ) -> Result<()> {
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

        let all = self
            .context
            .collation
            .resources();
            //.filter(|p| {
                //if !filter_active {
                    //return true;
                //}
                //p.starts_with(target)
            //})
            //.filter(|_p| {
                /*
                if let Some(ref manifest) = self.context.collation.manifest {
                    if let Some(ref resource) =
                        self.context.collation.all.get(*p)
                    {
                        match resource {
                            Resource::Page { ref target }
                            | Resource::File { ref target } => {
                                let file = p.to_path_buf();
                                if manifest.exists(&file)
                                    && !manifest.is_dirty(
                                        &file,
                                        &target.destination,
                                        false,
                                    )
                                {
                                    debug!("[NOOP] {}", file.display());
                                    return false;
                                }
                            }
                        }
                    }
                }
                */
                //true
            //});

        if parallel {
            let (tx, rx) = channel::unbounded();

            let context = &self.context;

            rayon::scope(|s| {
                for p in all {
                    let tx = tx.clone();
                    s.spawn(move |_t| {
                        let mut rt = tokio::runtime::Runtime::new().unwrap();
                        // NOTE: we pay a price for creating another runtime
                        // NOTE: inside the rayon thread but it gives us a
                        // NOTE: consistent futures based API
                        rt.block_on(async move {
                            let res = run::one(context, parser, p).await;
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
                        output.data.push(parse_data);
                    }
                }
            });

            if !errs.is_empty() {
                return Err(Error::Multi { errs });
            }
        } else {
            for p in all {
                if let Some(parse_data) = run::one(&self.context, parser, p).await? {
                    output.data.push(parse_data);
                }
            }
        }

        Ok(())
    }

    // Build all target paths
    pub async fn all(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        targets: &Vec<PathBuf>,
        output: &mut CompilerOutput,
    ) -> Result<()> {
        if let Some(hooks) = &self.context.config.hook {
            hook::run(
                Arc::clone(&self.context),
                hook::collect(
                    hooks.clone(),
                    hook::Phase::Before,
                    &self.context.options.settings.name,
                ),
            )?;
        }

        for p in targets {
            if p.is_file() {
                if let Some(parse_data) = run::one(&self.context, parser, &p).await? {
                    output.data.push(parse_data);
                }
            } else {
                self.build(parser, &p, output).await?;
            }
        }

        // Now compile the books
        // FIXME: refactor books
        if let Some(ref _book) = self.context.config.book {
            let livereload = crate::context::livereload().read().unwrap();
            self.book.all(&self.context.config, livereload.clone())?;
        }

        if let Some(hooks) = &self.context.config.hook {
            hook::run(
                Arc::clone(&self.context),
                hook::collect(
                    hooks.clone(),
                    hook::Phase::After,
                    &self.context.options.settings.name,
                ),
            )?;
        }

        Ok(())
    }
}
