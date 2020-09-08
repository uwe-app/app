use std::path::PathBuf;
use std::sync::Arc;

use crossbeam::channel;
use log::error;

use book::compiler::BookCompiler;

use collator::{Collate, Resource, ResourceOperation, ResourceTarget};

use crate::context::{BuildContext, CompilerOutput};
use crate::hook;
use crate::parser::Parser;
use crate::run::{self, ParseData};
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

    /// Handle a resource file depending upon the resource operation.
    pub async fn resource(
        &self,
        file: &PathBuf,
        target: &ResourceTarget,
    ) -> Result<()> {
        match target.operation {
            ResourceOperation::Noop => Ok(()),
            ResourceOperation::Copy => {
                run::copy(
                    file,
                    &target.get_output(self.context.collation.get_path()),
                )
                .await
            }
            ResourceOperation::Link => {
                run::link(
                    file,
                    &target.get_output(self.context.collation.get_path()),
                )
                .await
            }
            _ => Err(Error::InvalidResourceOperation(file.to_path_buf())),
        }
    }

    /// Build a single file, negotiates pages and resource files.
    pub async fn one(
        &self,
        parser: &Parser<'_>,
        file: &PathBuf,
    ) -> Result<Option<ParseData>> {
        let resource = self.context.collation.get_resource(file).unwrap();

        match resource {
            Resource::Page { ref target } => {
                if let Some(page) = self.context.collation.resolve(file) {
                    match target.operation {
                        ResourceOperation::Render => {
                            let rel =
                                page.file.as_ref().unwrap().target.clone();
                            let dest =
                                self.context.collation.get_path().join(&rel);

                            return run::parse(
                                Arc::clone(&self.context),
                                parser,
                                page.get_template(),
                                page,
                                &dest,
                            ).await;
                        }
                        _ => self.resource(file, target).await?,
                    }
                } else {
                    return Err(Error::PageResolve(file.to_path_buf()))
                }
            }
            Resource::File { ref target } => self.resource(file, target).await?,
        }

        Ok(None)
    }

    pub async fn build(
        &self,
        parser: &Parser<'_>,
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
            .resources()
            .filter(|p| {
                if !filter_active {
                    return true;
                }
                p.starts_with(target)
            })
            .filter(|_p| {
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
                true
            });

        //let mut data: Vec<ParseData> = Vec::new();

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
                        output.data.push(parse_data);
                    }
                }
            });

            if !errs.is_empty() {
                return Err(Error::Multi { errs });
            }
        } else {
            for p in all {
                if let Some(parse_data) = self.one(parser, p).await? {
                    output.data.push(parse_data);
                }
            }
        }

        Ok(())
    }

    // Build all target paths
    pub async fn all(
        &self,
        parser: &Parser<'_>,
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
                if let Some(parse_data) = self.one(parser, &p).await? {
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
