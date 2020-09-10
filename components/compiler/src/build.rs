use std::path::PathBuf;
use std::sync::Arc;

use crossbeam::channel;
use log::error;

use book::compiler::BookCompiler;

use collator::Collate;

use crate::context::{BuildContext, CompilerOutput};
use crate::parser::Parser;
use crate::run;
use crate::{Error, Result};

#[derive(Debug)]
pub struct Compiler {
    pub context: Arc<BuildContext>,
    //pub book: BookCompiler,
}

impl Compiler {
    pub fn new(context: Arc<BuildContext>) -> Self {
        //let book = BookCompiler::new(
        //context.options.source.clone(),
        //context.options.base.clone(),
        //context.options.settings.is_release(),
        //);

        Self { context }
    }

    pub async fn build<F>(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        output: &mut CompilerOutput,
        filter: F,
    ) -> Result<()>
    where
        F: FnMut(&&Arc<PathBuf>) -> bool + Send,
    {
        let parallel = self.context.options.settings.is_parallel();

        // TODO: support allowing this in the settings
        let fail_fast = true;

        let it = self.context.collation.resources().filter(filter);

        if parallel {
            let (tx, rx) = channel::unbounded();
            let context = &self.context;

            rayon::scope(|s| {
                for p in it {
                    let tx = tx.clone();
                    s.spawn(move |_t| {
                        // NOTE: we pay a price for creating another runtime
                        // NOTE: inside the rayon thread but it gives us a
                        // NOTE: consistent futures based API
                        let mut rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async move {
                            let res = run::one(context, parser, p).await;
                            if fail_fast && res.is_err() {
                                error!("{}", res.err().unwrap());
                                panic!("Build failed");
                            } else {
                                tx.send((p, res)).unwrap();
                            }
                        });
                    })
                }
            });

            drop(tx);

            let mut errs: Vec<Error> = Vec::new();
            rx.iter().for_each(|(p, r)| {
                if r.is_err() {
                    errs.push(r.err().unwrap());
                } else {
                    let res = r.unwrap();
                    if let Some(parse_data) = res {
                        output.data.push(parse_data);
                    }
                }
                output.files.push(Arc::clone(p));
            });

            if !errs.is_empty() {
                return Err(Error::Multi { errs });
            }
        } else {
            for p in it {
                if let Some(parse_data) =
                    run::one(&self.context, parser, p).await?
                {
                    output.data.push(parse_data);
                    output.files.push(Arc::clone(p));
                }
            }
        }

        Ok(())
    }

    // Build all target paths
    /*
    pub async fn all(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        targets: &Vec<PathBuf>,
        output: &mut CompilerOutput,
    ) -> Result<()> {

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

        Ok(())
    }
    */
}
