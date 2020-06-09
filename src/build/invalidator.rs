use std::path::PathBuf;

use super::Builder;
use super::context::Context;
use super::loader;
use super::matcher;

use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use crate::{
    Error,
    DATA_TOML,
    LAYOUT_HBS
};

use log::{info, debug, error};

use super::watch;

#[derive(Debug)]
pub struct Invalidation {
    data: bool,
    layout: bool,
    paths: Vec<PathBuf>,
}

/*
 *  Invalidation rules.
 *
 *  1) Resources are ignored as they are symbolically linked.
 *  2) Assets trigger a copy of the changed asset and a rebuild of all pages.
 *  3) Changes to data.toml trigger a rebuild of all pages.
 *  4) Changes to files in a `source` directory for a hook should run the hook again.
 */
pub struct Invalidator<'a> {
    context: &'a Context,
    builder: Builder<'a>,
}

impl<'a> Invalidator<'a> {
    pub fn new(context: &'a Context, builder: Builder<'a>) -> Self {
        Self { context, builder }
    }

    pub fn start(&mut self, from: PathBuf, tx: Sender<Message>) -> Result<(), Error> {
        #[cfg(feature = "watch")]
        let watch_result = watch::start(&from.clone(), move |paths, source_dir| {
            info!("changed({}) in {}", paths.len(), source_dir.display());
            debug!("files changed: {:?}", paths);
            if let Ok(invalidation) = self.get_invalidation(paths) {
                debug!("invalidation {:?}", invalidation);
                if let Err(e) = self.invalidate(&from, invalidation) {
                    error!("{}", e);
                }
                self.builder.save_manifest()?;
                let _ = tx.send(Message::text("reload"));
            }

            Ok(())
        });

        if let Err(e) = watch_result {
            return Err(e)
        }

        Ok(())
    }

    fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Invalidation, Error> {
        let mut invalidation = Invalidation{
            layout: false,
            data: false,
            paths: Vec::new()
        };

        let mut src = self.context.options.source.clone();
        if !src.is_absolute() {
            if let Ok(cwd) = std::env::current_dir() {
                src = cwd.clone();
                src.push(&self.context.options.source);
            }
        }

        // TODO: handle data.toml files???
        // TODO: handle layout file change - find dependents???
        // TODO: handle partial file changes - find dependents???

        let mut data_file = src.clone();
        data_file.push(DATA_TOML);

        let mut layout_file = src.clone();
        layout_file.push(LAYOUT_HBS);

        for path in paths {
            if path == data_file {
                invalidation.data = true;
            }else if path == layout_file {
                invalidation.layout = true;
            } else {
                if let Some(name) = path.file_name() {
                    let nm = name.to_string_lossy().into_owned();
                    if nm.starts_with(".") {
                        continue;
                    }
                }

                // Prefer relative paths, makes the output much 
                // easier to read
                if let Ok(cwd) = std::env::current_dir() {
                    if let Ok(p) = path.strip_prefix(cwd) {
                        invalidation.paths.push((*p).to_path_buf());
                    } else {
                        invalidation.paths.push(path);
                    }
                } else {
                    invalidation.paths.push(path);
                }
            }
        }

        Ok(invalidation)
    }

    fn invalidate(&mut self, target: &PathBuf, invalidation: Invalidation) -> Result<(), Error> {
        // FIXME: find out which section of the data.toml changed
        // FIXME: and ensure only those pages are invalidated
        
        // Should we invalidate everything?
        let mut all = false;

        if invalidation.data {
            info!("reload {}", DATA_TOML);
            if let Err(e) = loader::reload(&self.context.options) {
                error!("{}", e); 
            } else {
                all = true;
            }
        }

        if invalidation.layout {
            all = true;
        }

        if all {
            return self.builder.build(target, true);
        } else {
        
            for path in invalidation.paths {
                let file_type = matcher::get_type(&path, &self.context.config.extension.as_ref().unwrap());
                if let Err(e) = self.builder.process_file(&path, file_type, false) {
                    return Err(e)
                }
            }
        }
        Ok(())
    }
}
