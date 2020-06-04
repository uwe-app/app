use std::path::Path;
use std::convert::AsRef;

use serde::{Deserialize, Serialize};
use toml;

use crate::command::build::BuildOptions;

use crate::{utils, Error, SITE_TOML};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub build: Option<BuildOptions>,
}

pub fn load_config<P: AsRef<Path>>(p: P) -> Result<Config, Error> {
    let file = p.as_ref();

    println!("loading config from {:?}", p.as_ref().display());

    if file.exists() && file.is_file() {
        let content = utils::read_string(file)?;
        let cfg: Config = toml::from_str(&content)?;
        return Ok(cfg);
    }

    return Ok(Config::empty())
}

impl Config {
    pub fn empty() -> Self {
        Config {
            build: None,
        }
    }

    pub fn new<P: AsRef<Path>>(source: P) -> Self {
        let pth = source.as_ref().to_path_buf();
        for p in pth.ancestors() {
            let mut pb = p.to_path_buf();
            pb.push(SITE_TOML);
            if pb.exists() && pb.is_file() {
                if let Ok(result) = load_config(pb) {
                    println!("returning loaded config {:?}", result);
                    return result;
                } else {
                    println!("Error loading config");
                }
            }
        } 
        Config::empty()
    }

    pub fn for_build(&mut self, options: BuildOptions) {
        // FIXME: merge option
        self.build = Some(options);
        println!("load options for build");
    }
}
