use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use toml;

use crate::command::build::BuildOptions;

use crate::{utils, Error, SITE_TOML};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub build: BuildConfig,
    pub serve: ServeConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    pub source: PathBuf,
    pub target: PathBuf,
    pub strict: bool,
    pub html_extension: bool,
    pub extensions: BTreeMap<String, String>,
    pub follow_links: bool,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
}

pub fn load_config<P: AsRef<Path>>(p: P) -> Result<Config, Error> {
    let file = p.as_ref();

    println!("loading config from {:?}", p.as_ref().display());

    if file.exists() && file.is_file() {
        let content = utils::read_string(file)?;
        let cfg: Config = toml::from_str(&content)?;
        println!("loaded config {:?}", cfg);
        return Ok(cfg);
    }

    return Ok(Config::new())
}

impl Config {
    pub fn new() -> Self {
        Config {
            build: BuildConfig {
                source: Path::new("site").to_path_buf(),
                target: Path::new("build").to_path_buf(),
                strict: true,
                ..Default::default()
            },
            serve: ServeConfig {
                host: String::from("localhost"),
                port: 3000,
            },
        }
    }

    pub fn load<P: AsRef<Path>>(source: P) -> Result<Self, Error> {
        let pth = source.as_ref().to_path_buf();

        if pth.ends_with(SITE_TOML) {
            return load_config(pth)
        }

        for p in pth.ancestors() {
            let mut pb = p.to_path_buf();
            pb.push(SITE_TOML);
            if pb.exists() && pb.is_file() {
                return load_config(pb)
            }
        } 
        Err(Error::new(
            format!("No configuration found for {}", pth.display())))
    }
}
