use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;
use std::collections::BTreeMap;

use url::Url;

use serde_with::skip_serializing_none;
use serde::{Deserialize, Serialize};
use toml;

use unic_langid::LanguageIdentifier;

use crate::{utils, Error, MD, HTML};
use crate::build::page::Page;

static SITE_TOML: &str = "site.toml";
static LAYOUT_HBS: &str = "layout.hbs";

static PAGES: &str = "page.toml";

static ASSETS: &str = "assets";
static PARTIALS: &str = "partials";
static GENERATORS: &str = "generators";
static RESOURCES: &str = "resources";

static HOST: &str = "localhost";
static PORT: u16 = 3000;

static LANG: &str = "en";
static LOCALES: &str = "locales";


use log::debug;

fn resolve_project<P: AsRef<Path>>(f: P) -> Option<PathBuf> {
    let file = f.as_ref();
    if let Some(p) = file.parent() {
        return Some(p.to_path_buf());
    } else {
        if let Ok(cwd) = std::env::current_dir() {
            return Some(cwd)
        }
    }
    None
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub lang: String,
    pub host: Option<String>,
    pub file: Option<PathBuf>,
    pub project: Option<PathBuf>,
    pub build: Option<BuildConfig>,
    pub workspace: Option<WorkspaceConfig>,
    pub serve: Option<ServeConfig>,
    pub book: Option<BookConfig>,
    pub extension: Option<ExtensionConfig>,
    pub fluent: Option<FluentConfig>,
    pub hook: Option<BTreeMap<String, HookConfig>>,
    pub page: Option<Page>,

    #[serde(skip)]
    pub url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceConfig {
    pub members: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lang: String::from(LANG),
            host: Some(String::from(HOST)),
            url: None,
            file: None,
            project: None,
            build: Some(Default::default()),
            workspace: None,
            extension: Some(Default::default()),
            fluent: None,
            book: None,
            serve: Some(Default::default()),
            hook: None,
            page: Some(Default::default()),
        } 
    }
}

impl Config {

    pub fn load_config<P: AsRef<Path>>(p: P) -> Result<Self, Error> {
        let file = p.as_ref();
        debug!("load {:?}", p.as_ref().display());
        if let Some(base) = file.parent() {
            if file.exists() && file.is_file() {

                let content = utils::read_string(file)?;
                let mut cfg: Config = toml::from_str(&content)?;

                cfg.project = resolve_project(&file);
                if cfg.project.is_none() {
                    return Err(
                        Error::new(
                            format!("Failed to resolve project directory for {}", file.display())));
                }

                // Must be a canonical path
                let path = file.canonicalize()?;
                cfg.file = Some(path.to_path_buf());

                // Ensure that lang is a valid identifier
                let _: LanguageIdentifier = cfg.lang.parse()?;

                if let Some(host) = cfg.host.as_ref() {
                    let mut host = host.clone();
                    host = host.trim_start_matches("http://").to_string();
                    host = host.trim_start_matches("https://").to_string();

                    let mut url_host = String::from("https://");
                    url_host.push_str(&host);

                    let url = Url::parse(&url_host)?;
                    cfg.url = Some(url);
                }

                //if cfg.page.is_none() {
                    //cfg.page = Some(Default::default());
                //}

                if cfg.fluent.is_some() {
                    let mut fluent = cfg.fluent.as_mut().unwrap();
                    if fluent.fallback.is_some() {
                        fluent.fallback_id = fluent.fallback.as_ref().unwrap().parse()?;
                    }
                    if fluent.locales.is_none() {
                        fluent.locales = Some(PathBuf::from(LOCALES)); 
                    }
                }

                // Assume default build settings for the site
                if cfg.build.is_none() {
                    cfg.build = Some(Default::default());
                }

                let mut build = cfg.build.as_mut().unwrap();

                if build.source.is_relative() {
                    let mut bp = base.to_path_buf(); 
                    bp.push(&build.source);
                    build.source = bp;
                }

                if build.target.is_relative() {
                    let mut bp = base.to_path_buf(); 
                    bp.push(&build.target);
                    build.target = bp;
                }

                if let Some(hooks) = cfg.hook.as_mut() {
                    for (k, v) in hooks.iter_mut() {
                        if v.path.is_none() {
                            v.path = Some(k.clone());
                        } 
                        if v.stdout.is_none() {
                            v.stdout = Some(true);
                        }
                        if v.stderr.is_none() {
                            v.stderr = Some(true);
                        }
                    }
                } else {
                    // Create a default value so we can always
                    // unwrap()
                    cfg.hook = Some(BTreeMap::new());
                }

                return Ok(cfg);
            }
        }
        return Ok(Default::default())
    }

    pub fn load<P: AsRef<Path>>(source: P, walk_ancestors: bool) -> Result<Self, Error> {
        let mut pth = source.as_ref().to_path_buf();
        if pth.is_file() && pth.ends_with(SITE_TOML) {
            return Config::load_config(pth)
        } else if pth.is_dir() {
            pth.push(SITE_TOML);
            if pth.is_file() && pth.exists() {
                return Config::load_config(pth)
            }
        }

        if walk_ancestors {
            for p in pth.ancestors() {
                let mut pb = p.to_path_buf();
                pb.push(SITE_TOML);
                if pb.exists() && pb.is_file() {
                    return Config::load_config(pb)
                }
            } 
        }
        Err(Error::new(format!("No configuration found for {}", pth.display())))
    }

    pub fn get_project(&self) -> PathBuf {
        self.project.as_ref().unwrap().clone()
    }

    pub fn get_layout_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let mut pth = source.as_ref().to_path_buf();
        pth.push(LAYOUT_HBS);
        pth 
    }

    pub fn get_page_data_path(&self) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let pages = build.pages.as_ref().unwrap();
        let mut pth = self.project.as_ref().unwrap().clone();
        pth.push(pages);
        pth 
    }

    pub fn get_locales<P: AsRef<Path>>(&self, source: P) -> Option<PathBuf> {
        if let Some(fluent) = &self.fluent {
            if let Some(locales) = &fluent.locales {
                let mut pth = source.as_ref().to_path_buf();
                pth.push(locales);
                return Some(pth) 
            } 
        }
        None
    }

    pub fn get_assets_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let assets = build.assets.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(assets);
        pth 
    }

    pub fn get_partials_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let partial = build.partials.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(partial);
        pth 
    }

    pub fn get_generators_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let generator = build.generators.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(generator);
        pth 
    }

    pub fn get_resources_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let resource = build.resources.as_ref().unwrap();
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct BuildConfig {
    pub source: PathBuf,
    pub target: PathBuf,
    pub strict: Option<bool>,
    pub pages: Option<PathBuf>,
    pub assets: Option<PathBuf>,
    pub partials: Option<PathBuf>,
    pub generators: Option<PathBuf>,
    pub resources: Option<PathBuf>,
    pub clean_url: Option<bool>,
    pub follow_links: Option<bool>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        BuildConfig {
            source: PathBuf::from("site"),
            target: PathBuf::from("build"),
            strict: Some(true),
            pages: Some(PathBuf::from(PAGES)),
            assets: Some(PathBuf::from(ASSETS)),
            partials: Some(PathBuf::from(PARTIALS)),
            generators: Some(PathBuf::from(GENERATORS)),
            resources: Some(PathBuf::from(RESOURCES)),
            clean_url: Some(true),
            follow_links: Some(true),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FluentConfig {
    pub fallback: Option<String>,
    pub redirect: Option<String>,
    pub locales: Option<PathBuf>,
    pub shared: Option<String>,
    #[serde(skip)]
    pub fallback_id: LanguageIdentifier,
}

impl Default for FluentConfig {
    fn default() -> Self {
        Self {
            fallback: Some(String::from(LANG)),
            redirect: Some(String::from(LANG)),
            locales: Some(PathBuf::from(LOCALES)),
            shared: Some(String::from("core.ftl")),
            fallback_id: String::from(LANG).parse().unwrap(),
        } 
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            host: String::from(HOST),
            port: PORT,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BookConfig {
    pub theme: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionConfig {
    pub render: Vec<String>,
    pub map: BTreeMap<String, String>,
    pub markdown: Vec<String>,
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        let mut ext_map: BTreeMap<String, String> = BTreeMap::new();
        ext_map.insert(String::from(MD), String::from(HTML));
        ExtensionConfig {
            render: vec![String::from(MD), String::from(HTML)],
            map: ext_map,
            markdown: vec![String::from(MD)],
        } 
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HookConfig {
    pub path: Option<String>,
    pub args: Option<Vec<String>>,
    pub source: Option<PathBuf>,
    pub stdout: Option<bool>,
    pub stderr: Option<bool>,
}

impl HookConfig {
    pub fn get_source_path<P: AsRef<Path>>(&self, source: P) -> Option<PathBuf> {
        if let Some(src) = self.source.as_ref() {
            let mut pth = source.as_ref().to_path_buf();
            pth.push(src);
            return Some(pth) 
        }
        None
    }
}
