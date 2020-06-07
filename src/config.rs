use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use toml;

//use crate::command::build::BuildOptions;

use crate::{utils, Error, SITE_TOML, MD, HTML};

use log::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub build: BuildConfig,
    pub extensions: Option<ExtensionsConfig>,
    pub serve: ServeConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    pub source: PathBuf,
    pub target: PathBuf,
    pub strict: bool,
    pub html_extension: bool,
    pub follow_links: bool,
}

impl BuildConfig {
    pub fn new() -> Self {
        BuildConfig {
            source: Path::new("site").to_path_buf(),
            target: Path::new("build").to_path_buf(),
            strict: true,
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ExtensionsConfig {
    pub render: Vec<String>,
    pub map: BTreeMap<String, String>,
    pub markdown: Vec<String>,
}

impl ExtensionsConfig {
    pub fn new() -> Self {
        let mut ext_map: BTreeMap<String, String> = BTreeMap::new();
        ext_map.insert(String::from(MD), String::from(HTML));

        ExtensionsConfig {
            render: vec![String::from(MD), String::from(HTML)],
            map: ext_map,
            markdown: vec![String::from(MD)],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
}

impl ServeConfig {
    pub fn new() -> Self {
        ServeConfig {
            host: String::from("localhost"),
            port: 3000,
        }
    }
}

pub fn load_config<P: AsRef<Path>>(p: P) -> Result<Config, Error> {
    let file = p.as_ref();

    debug!("load {:?}", p.as_ref().display());

    if let Some(base) = file.parent() {
        if file.exists() && file.is_file() {
            let content = utils::read_string(file)?;
            let mut cfg: Config = toml::from_str(&content)?;

            if cfg.build.source.is_relative() {
                let mut bp = base.to_path_buf(); 
                bp.push(&cfg.build.source);
                cfg.build.source = bp;
            }

            if cfg.build.target.is_relative() {
                let mut bp = base.to_path_buf(); 
                bp.push(&cfg.build.target);
                cfg.build.target = bp;
            }

            if cfg.extensions.is_none() {
                cfg.extensions = Some(ExtensionsConfig::new());
            }

            return Ok(cfg);
        }
    }

    return Ok(Config::new())
}

impl Config {
    pub fn new() -> Self {
        Config {
            build: BuildConfig::new(),
            extensions: Some(ExtensionsConfig::new()),
            serve: ServeConfig::new(),
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
