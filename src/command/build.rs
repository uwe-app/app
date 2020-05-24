use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::build::Builder;
use crate::build::loader;
use crate::command::serve::*;
use crate::{Error, LIVE_RELOAD_ENDPOINT};

use log::{info, debug, error};

#[derive(Debug, Serialize, Deserialize)]
pub enum BuildTag {
    Custom(String),
    Debug,
    Release
}

impl BuildTag {
    pub fn get_path_name(&self) -> String {
        match self {
            BuildTag::Debug => return "debug".to_owned(),
            BuildTag::Release => return "release".to_owned(),
            BuildTag::Custom(s) => return s.to_owned()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildOptions {
    pub source: PathBuf,
    pub output: PathBuf,
    pub target: PathBuf,
    pub directory: Option<PathBuf>,
    pub max_depth: Option<usize>,
    pub release: bool,
    pub follow_links: bool,
    pub strict: bool,
    pub clean_url: bool,
    pub tag: BuildTag,
    pub live: bool,
    pub livereload: Option<String>,
    pub host: String,
    pub port: String,
}

pub fn build(mut options: BuildOptions) -> Result<(), Error> {
    if let Err(e) = loader::load(&options) {
        return Err(e)
    }

    if options.live {
        let url = format!("ws://{}:{}/{}", options.host, options.port, LIVE_RELOAD_ENDPOINT);
        options.livereload = Some(url);
    }

    let mut builder = Builder::new(&options);
    builder.build()?;

    if options.live {
        let opts = ServeOptions::new(
            options.target.clone(),
            options.source.clone(),
            options.host.to_owned(),
            options.port.to_owned());

        serve(opts, move |paths, source_dir| {
            info!("changed({}) in {}", paths.len(), source_dir.display());
            debug!("files changed: {:?}", paths);

            let result = builder.build_files(paths);
            match result {
                Err(e) => {
                    error!("{}", e);
                }
                _ => {}
            }

            Ok(())
        })?;
    }

    Ok(())
}

