use std::collections::HashMap;
use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use url::Url;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use toml;

use log::debug;
use unic_langid::LanguageIdentifier;

use utils;

use super::profile::{ProfileName, ProfileSettings};
use super::page::{Author, Page};
use super::Error;
use super::indexer::{IndexRequest, DataSource};

pub static SITE: &str = "site";
pub static BUILD: &str = "build";
pub static LOCALES: &str = "locales";
pub static CORE_FTL: &str = "core.ftl";
pub static MAIN_FTL: &str = "main.ftl";
pub static SITE_TOML: &str = "site.toml";
pub static LANG_KEY: &str = "lang";
pub static HOST_KEY: &str = "host";
pub static FLUENT_KEY: &str = "fluent";
pub static FALLBACK_KEY: &str = "fallback";
pub static SHARED_KEY: &str = "shared";
pub static REDIRECT_KEY: &str = "redirect";
pub static HTML: &str = "html";
pub static INDEX_STEM: &str = "index";
pub static LAYOUT_HBS: &str = "layout.hbs";
pub static MD: &str = "md";
pub static TOML: &str = "toml";
pub static BOOK_TOML: &str = "book.toml";
pub static PAGE_DATA: &str = "page.toml";
pub static ASSETS: &str = "assets";
pub static PARTIALS: &str = "partials";
pub static INCLUDES: &str = "includes";
pub static DATASOURCES: &str = "data-sources";
pub static RESOURCES: &str = "resources";
pub static LANG: &str = "en";
pub static LIVERELOAD_FILE: &str = "__livereload.js";

pub static HOST: &str = "localhost";
pub static PORT: u16 = 8888;

type RedirectConfig = HashMap<String, String>;

fn resolve_cwd() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        return Some(cwd);
    }
    return None;
}

fn resolve_project<P: AsRef<Path>>(f: P) -> Option<PathBuf> {
    let file = f.as_ref();
    if let Some(p) = file.parent() {
        // Hooks need a canonical path for resolving relative
        // executables and if we allow the empty string the
        // call to canonical() in the hook builder will fail
        if p == PathBuf::from("") {
            return resolve_cwd();
        }

        return Some(p.to_path_buf());
    }

    resolve_cwd()
}

pub fn parse_language<S: AsRef<str>>(lang: S) -> Result<LanguageIdentifier, Error> {
    let id: LanguageIdentifier = lang.as_ref().parse()?;
    Ok(id)
}

pub fn parse_host<S: AsRef<str>>(host: S) -> Result<Url, Error> {
    let mut src = host.as_ref().clone().to_string();
    // It's ok if people want to declare a scheme but we don't
    // want one for the host
    src = src
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .to_string();

    // Check host can be parsed as a valid URL
    // and return the parsed URL
    let url_host = format!("https://{}", src);
    Ok(Url::parse(&url_host)?)
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub lang: String,
    pub host: String,
    pub build: Option<ProfileSettings>,
    pub workspace: Option<WorkspaceConfig>,
    pub serve: Option<ServeConfig>,
    pub book: Option<BookConfig>,
    pub fluent: Option<FluentConfig>,
    pub hook: Option<HashMap<String, HookConfig>>,
    pub node: Option<NodeConfig>,
    pub page: Option<Page>,
    pub redirect: Option<RedirectConfig>,
    pub date: Option<DateConfig>,
    pub link: Option<LinkConfig>,
    pub profile: Option<HashMap<String, ProfileSettings>>,
    pub publish: Option<PublishConfig>,
    pub index: Option<HashMap<String, IndexRequest>>,
    pub authors: Option<HashMap<String, Author>>,

    pub collate: Option<HashMap<String, DataSource>>,
    pub minify: Option<MinifyConfig>,
    pub livereload: Option<LiveReload>,

    #[serde(skip)]
    pub file: Option<PathBuf>,

    #[serde(skip)]
    pub project: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lang: String::from(LANG),
            host: String::from(HOST),
            file: None,
            project: None,
            build: Some(Default::default()),
            workspace: None,
            fluent: None,
            book: None,
            serve: Some(Default::default()),
            hook: None,
            node: Some(Default::default()),
            page: Some(Default::default()),
            redirect: None,
            date: Some(Default::default()),
            link: Some(Default::default()),
            profile: Some(Default::default()),
            publish: Some(Default::default()),
            index: None,
            authors: None,
            collate: None,
            minify: None,
            livereload: Some(Default::default()),
        }
    }
}

impl Config {
    pub fn load_config<P: AsRef<Path>>(p: P) -> Result<Self, Error> {
        let file = p.as_ref();
        debug!("load {:?}", p.as_ref().display());
        if let Some(base) = file.parent() {
            if file.exists() && file.is_file() {
                let content = utils::fs::read_string(file)?;
                let mut cfg: Config = toml::from_str(&content)?;

                cfg.project = resolve_project(&file);
                if cfg.project.is_none() {
                    return Err(Error::ProjectResolve(file.to_path_buf()));
                }

                // Must be a canonical path
                let path = file.canonicalize()?;
                cfg.file = Some(path.to_path_buf());

                // Ensure that lang is a valid identifier
                parse_language(&cfg.lang)?;

                // Ensure the host is a valid Url
                parse_host(&cfg.host)?;

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

                if let Some(ref book) = cfg.book {
                    let book_paths = book.get_paths(&build.source);
                    for mut p in book_paths {
                        if !p.exists() || !p.is_dir() {
                            return Err(Error::NotDirectory(p));
                        }

                        p.push(BOOK_TOML);
                        if !p.exists() || !p.is_file() {
                            return Err(Error::NoBookConfig(p));
                        }
                    }
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
                    cfg.hook = Some(HashMap::new());
                }

                if let Some(date) = cfg.date.as_mut() {
                    let mut datetime_formats = HashMap::new();
                    datetime_formats.insert("date-short".to_string(), "%F".to_string());
                    datetime_formats.insert("date-medium".to_string(), "%a %b %e %Y".to_string());
                    datetime_formats.insert("date-long".to_string(), "%A %B %e %Y".to_string());

                    datetime_formats.insert("time-short".to_string(), "%R".to_string());
                    datetime_formats.insert("time-medium".to_string(), "%X".to_string());
                    datetime_formats.insert("time-long".to_string(), "%r".to_string());

                    datetime_formats.insert("datetime-short".to_string(), "%F %R".to_string());
                    datetime_formats
                        .insert("datetime-medium".to_string(), "%a %b %e %Y %X".to_string());
                    datetime_formats
                        .insert("datetime-long".to_string(), "%A %B %e %Y %r".to_string());

                    for (k, v) in datetime_formats {
                        if !date.formats.contains_key(&k) {
                            date.formats.insert(k, v);
                        }
                    }

                    // FIXME: validate date time format specifiers
                }

                if let Some(collators) = cfg.collate.as_mut() {
                    for(_, v) in collators {
                        if let Some(ref from) = v.from {
                            if from.is_relative() {
                                let mut tmp = build.source.clone();
                                tmp.push(from);
                                v.from = Some(tmp);
                            }
                        }
                    }
                }

                let mut livereload = cfg.livereload.as_mut().unwrap();
                if livereload.file.is_none() {
                    livereload.file = Some(PathBuf::from(LIVERELOAD_FILE));
                }

                let mut link = cfg.link.as_mut().unwrap();
                if let Some(ref catalog) = link.catalog {
                    let catalog_path = build.source.clone().join(catalog);
                    let content = utils::fs::read_string(&catalog_path)
                        .map_err(|_| Error::LinkCatalog(catalog_path))?;
                    link.catalog_content = Some(content);
                }

                return Ok(cfg);
            }
        }
        return Ok(Default::default());
    }

    pub fn load<P: AsRef<Path>>(source: P, walk_ancestors: bool) -> Result<Self, Error> {
        let mut pth = source.as_ref().to_path_buf();
        if pth.is_file() && pth.ends_with(SITE_TOML) {
            return Config::load_config(pth);
        } else if pth.is_file() {
            if let Some(ext) = pth.extension() {
                if ext == TOML {
                    return Config::load_config(pth);
                } 
            }
        } else if pth.is_dir() {
            pth.push(SITE_TOML);
            if pth.is_file() && pth.exists() {
                return Config::load_config(pth);
            }
        }

        if walk_ancestors {
            for p in pth.ancestors() {
                let mut pb = p.to_path_buf();
                pb.push(SITE_TOML);
                if pb.exists() && pb.is_file() {
                    return Config::load_config(pb);
                }
            }
        }

        // Better error message when looking in the cwd
        if pth == PathBuf::from("") {
            if let Some(cwd) = resolve_cwd() {
                pth = cwd;
            }
        }

        Err(Error::NoSiteConfig(pth))
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
                return Some(pth);
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

    pub fn get_includes_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let partial = build.includes.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(partial);
        pth
    }

    pub fn get_partials_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let partial = build.partials.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(partial);
        pth
    }

    pub fn get_datasources_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let build = self.build.as_ref().unwrap();
        let datasources = build.data_sources.as_ref().unwrap();
        let mut pth = source.as_ref().to_path_buf();
        pth.push(datasources);
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
pub struct WorkspaceConfig {
    pub members: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookConfig {
    pub theme: PathBuf,
    //pub groups: Vec<String>,
    #[serde(flatten)]
    pub members: HashMap<String, HashMap<String, BookItem>>,
}

impl BookConfig {
    pub fn get_paths<P: AsRef<Path>>(&self, base: P) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        let source = base.as_ref().to_path_buf();
        for (_, map) in &self.members {
            for (_, value) in map {
                let mut tmp = source.clone();
                tmp.push(value.path.clone());
                out.push(tmp);
            }
        }
        out
    }

    pub fn find<P: AsRef<Path>>(&self, path: P) -> Option<BookItem> {
        let needle = path.as_ref().to_path_buf();
        for (_, map) in &self.members {
            for (_, value) in map {
                if value.path == needle {
                    return Some(value.clone());
                }
            }
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BookItem {
    pub path: PathBuf,
    pub draft: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FluentConfig {
    pub fallback: Option<String>,
    pub locales: Option<PathBuf>,
    pub shared: Option<String>,
    #[serde(skip)]
    pub fallback_id: LanguageIdentifier,
}

impl Default for FluentConfig {
    fn default() -> Self {
        Self {
            fallback: Some(String::from(LANG)),
            locales: Some(PathBuf::from(LOCALES)),
            shared: Some(String::from(CORE_FTL)),
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

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HookConfig {
    pub path: Option<String>,
    pub args: Option<Vec<String>>,
    pub source: Option<PathBuf>,
    pub stdout: Option<bool>,
    pub stderr: Option<bool>,
    // Marks the hook to run after a build
    pub after: Option<bool>,
    // Only run for these profiles
    pub profiles: Option<Vec<ProfileName>>,
}

impl HookConfig {
    pub fn get_source_path<P: AsRef<Path>>(&self, source: P) -> Option<PathBuf> {
        if let Some(src) = self.source.as_ref() {
            let mut pth = source.as_ref().to_path_buf();
            pth.push(src);
            return Some(pth);
        }
        None
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct NodeConfig {
    // Allow custom mappings for NODE_ENV
    pub debug: Option<String>,
    pub release: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateConfig {
    pub formats: HashMap<String, String>,
}

impl Default for DateConfig {
    fn default() -> Self {
        Self {
            formats: HashMap::new(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkConfig {
    /// Explicit list of paths that are allowed, should
    /// not begin with a forward slash
    pub allow: Option<Vec<String>>,
    /// The link helper should verify links
    pub verify: Option<bool>,
    /// The link helper should make links relative
    pub relative: Option<bool>,
    /// Catalog for markdown documents
    pub catalog: Option<PathBuf>,
    #[serde(skip)]
    pub catalog_content: Option<String>,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            allow: None,
            verify: Some(true),
            relative: Some(true),
            catalog: None,
            catalog_content: None,
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MinifyConfig {
    pub html: Option<MinifyFormat>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MinifyFormat {
    pub profiles: Vec<ProfileName>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LiveReload {
    pub notify: Option<bool>,

    // This is undocumented but here if it must be used
    pub file: Option<PathBuf>,
}

impl Default for LiveReload {
    fn default() -> Self {
        Self {
            notify: Some(true),
            file: Some(PathBuf::from(LIVERELOAD_FILE)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PublishConfig {
    pub aws: Option<AwsPublishConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AwsPublishConfig {
    pub credentials: String,
    pub region: String,
    pub environments: HashMap<String, AwsPublishEnvironment>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AwsPublishEnvironment {
    pub prefix: Option<String>,
    pub bucket: Option<String>,
}
