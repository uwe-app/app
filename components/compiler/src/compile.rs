use std::path::PathBuf;
use std::sync::Arc;

use crossbeam::channel;
use log::error;

use collator::Collate;

use crate::{
    context::{BuildContext, CompilerOutput},
    parser::Parser,
    run, Error, Result,
};

pub async fn compile<F>(
    context: &BuildContext,
    parser: &Box<impl Parser + Send + Sync + ?Sized>,
    output: &mut CompilerOutput,
    filter: F,
) -> Result<()>
where
    F: FnMut(&&Arc<PathBuf>) -> bool + Send,
{
    let parallel = context.options.settings.is_parallel();

    // TODO: support allowing this in the settings
    let fail_fast = true;

    let collation = &*context.collation.read().unwrap();
    let it = collation.resources().filter(filter);

    if parallel {
        let (tx, rx) = channel::unbounded();
        //let context = &self.context;

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
            if let Some(parse_data) = run::one(context, parser, p).await? {
                output.data.push(parse_data);
                output.files.push(Arc::clone(p));
            }
        }
    }

    Ok(())
}
