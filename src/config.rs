use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use toml;

use crate::{utils, Error, MD, HTML};

static SITE_TOML: &str = "site.toml";
static PARTIAL: &str = "partial";
static GENERATOR: &str = "generator";
static RESOURCE: &str = "resource";

use log::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub file: Option<PathBuf>,
    pub build: BuildConfig,
    pub serve: ServeConfig,
    pub book: Option<BookConfig>,
    pub extension: Option<ExtensionConfig>,
}

impl Config {

    pub fn new() -> Self {
        Self {
            file: None,
            build: BuildConfig::new(),
            extension: Some(ExtensionConfig::new()),
            book: None,
            serve: ServeConfig::new(),
        }
    }

    pub fn load_config<P: AsRef<Path>>(p: P) -> Result<Self, Error> {
        let file = p.as_ref();
        debug!("load {:?}", p.as_ref().display());
        if let Some(base) = file.parent() {
            if file.exists() && file.is_file() {
                let content = utils::read_string(file)?;
                let mut cfg: Config = toml::from_str(&content)?;

                cfg.file = Some(file.to_path_buf());

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

                if cfg.build.strict.is_none() {
                    cfg.build.strict = Some(true);
                }

                if cfg.build.partial.is_none() {
                    cfg.build.partial = Some(PathBuf::from(PARTIAL));
                }

                if cfg.build.generator.is_none() {
                    cfg.build.generator = Some(PathBuf::from(GENERATOR));
                }

                if cfg.build.resource.is_none() {
                    cfg.build.resource = Some(PathBuf::from(RESOURCE));
                }

                if cfg.build.clean_url.is_none() {
                    cfg.build.clean_url= Some(true);
                }

                if cfg.extension.is_none() {
                    cfg.extension = Some(ExtensionConfig::new());
                }

                return Ok(cfg);
            }
        }
        return Ok(Config::new())
    }

    pub fn load<P: AsRef<Path>>(source: P) -> Result<Self, Error> {
        let pth = source.as_ref().to_path_buf();
        if pth.ends_with(SITE_TOML) {
            return Config::load_config(pth)
        }
        for p in pth.ancestors() {
            let mut pb = p.to_path_buf();
            pb.push(SITE_TOML);
            if pb.exists() && pb.is_file() {
                return Config::load_config(pb)
            }
        } 
        Err(Error::new(
            format!("No configuration found for {}", pth.display())))
    }

    pub fn get_partial_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let partial = self.build.partial.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(partial);
        pth 
    }

    pub fn get_generator_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let generator = self.build.generator.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(generator);
        pth 
    }

    pub fn get_resource_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let resource = self.build.resource.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(resource);
        pth 
    }

    pub fn get_book_theme_path<P: AsRef<Path>>(&self, source: P) -> Option<PathBuf> {
        if let Some(book) = &self.book {
            let mut pth = source.as_ref().to_path_buf();
            pth.push(book.theme.clone());
            return Some(pth);
        }
        None
    }

}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    pub source: PathBuf,
    pub target: PathBuf,
    pub strict: Option<bool>,
    pub partial: Option<PathBuf>,
    pub generator: Option<PathBuf>,
    pub resource: Option<PathBuf>,
    pub clean_url: Option<bool>,
    pub follow_links: Option<bool>,
}

impl BuildConfig {
    pub fn new() -> Self {
        BuildConfig {
            source: PathBuf::from("site"),
            target: PathBuf::from("build"),
            strict: Some(true),
            partial: Some(PathBuf::from(PARTIAL)),
            generator: Some(PathBuf::from(GENERATOR)),
            resource: Some(PathBuf::from(RESOURCE)),
            clean_url: Some(true),
            ..Default::default()
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
        Self {
            host: String::from("localhost"),
            port: 3000,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ExtensionConfig {
    pub render: Vec<String>,
    pub map: BTreeMap<String, String>,
    pub markdown: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BookConfig {
    pub theme: PathBuf,
}

impl ExtensionConfig {
    pub fn new() -> Self {
        let mut ext_map: BTreeMap<String, String> = BTreeMap::new();
        ext_map.insert(String::from(MD), String::from(HTML));
        Self {
            render: vec![String::from(MD), String::from(HTML)],
            map: ext_map,
            markdown: vec![String::from(MD)],
        }
    }
}

