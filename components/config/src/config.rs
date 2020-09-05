use std::collections::HashMap;
use std::path::{Path, PathBuf};

use url::Url;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use toml;

use log::debug;
use unic_langid::LanguageIdentifier;

use super::feed::FeedConfig;
use super::indexer::{DataBase, IndexRequest};
use super::page::{Author, Page};
use super::profile::{ProfileName, ProfileSettings};
use super::redirect::RedirectConfig;
use super::script::JavaScriptConfig;
use super::search::SearchConfig;
use super::style::StyleSheetConfig;
use super::syntax::SyntaxConfig;
use super::transform::TransformConfig;
use super::Error;

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
pub static INDEX_HTML: &str = "index.html";
pub static LAYOUT_HBS: &str = "layout.hbs";
pub static MD: &str = "md";
pub static TOML: &str = "toml";
pub static JSON: &str = "json";
pub static BOOK_TOML: &str = "book.toml";
pub static ASSETS: &str = "assets";
pub static PARTIALS: &str = "partials";
pub static INCLUDES: &str = "includes";
pub static DATASOURCES: &str = "data-sources";
pub static RESOURCES: &str = "resources";
pub static LANG: &str = "en";
pub static LIVERELOAD_FILE: &str = "__livereload.js";
pub static TAGS: &str = "tags";

/// Used when multiple virtual hosts and inferring
/// a sub-domain from the primary host name.
pub static HOST_DEV: &str = "loopback.space";

pub static ADDR: &str = "127.0.0.1";
pub static HOST: &str = "loopback.space";
pub static PORT: u16 = 8888;
pub static PORT_SSL: u16 = 8843;

pub static SCHEME_HTTPS: &str = "https:";
pub static SCHEME_HTTP: &str = "http:";
pub static SCHEME_DELIMITER: &str = "//";
pub static SCHEME_WSS: &str = "wss:";
pub static SCHEME_WS: &str = "ws:";

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
    let mut host = host.as_ref().clone().to_string();
    // It's ok if people want to declare a scheme but we don't
    // want one for the host
    host = host
        .trim_start_matches(SCHEME_HTTP)
        .trim_start_matches(SCHEME_HTTPS)
        .trim_start_matches(SCHEME_DELIMITER)
        .to_string();

    // Check host can be parsed as a valid URL
    // and return the parsed URL
    Ok(Url::parse(&crate::to_url_string(
        SCHEME_HTTPS,
        &host,
        None,
    ))?)
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub lang: String,
    pub host: String,

    // Host name when running locally which overrides the inferred
    // localhost subdomain
    pub localhost: Option<String>,

    pub build: Option<ProfileSettings>,
    pub workspace: Option<WorkspaceConfig>,
    pub book: Option<BookConfig>,
    pub fluent: Option<FluentConfig>,
    pub hook: Option<HashMap<String, HookConfig>>,
    pub node: Option<NodeConfig>,
    pub page: Option<Page>,
    pub pages: Option<HashMap<String, Page>>,
    pub redirect: Option<RedirectConfig>,
    pub date: Option<DateConfig>,
    pub link: Option<LinkConfig>,
    pub profile: Option<HashMap<String, ProfileSettings>>,
    pub publish: Option<PublishConfig>,
    pub index: Option<HashMap<String, IndexRequest>>,
    pub authors: Option<HashMap<String, Author>>,

    pub syntax: Option<SyntaxConfig>,
    pub transform: Option<TransformConfig>,
    pub search: Option<SearchConfig>,
    pub feed: Option<FeedConfig>,
    pub scripts: Option<JavaScriptConfig>,
    pub styles: Option<StyleSheetConfig>,

    pub db: Option<DataBase>,

    pub minify: Option<MinifyConfig>,
    pub livereload: Option<LiveReload>,

    pub series: Option<HashMap<String, SeriesConfig>>,

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
            localhost: None,
            build: Some(Default::default()),
            workspace: None,
            fluent: Some(Default::default()),
            book: None,
            hook: None,
            node: Some(Default::default()),
            page: Some(Default::default()),
            pages: None,
            redirect: None,
            date: Some(Default::default()),
            link: Some(Default::default()),
            profile: Some(Default::default()),
            publish: Some(Default::default()),
            index: None,
            authors: None,
            syntax: None,
            transform: Some(Default::default()),
            search: None,
            feed: None,
            scripts: None,
            styles: None,
            db: None,
            minify: None,
            livereload: Some(Default::default()),
            series: None,

            file: None,
            project: None,
        }
    }
}

impl Config {
    pub fn get_local_host_name(&self, infer_from_host: bool) -> String {
        if let Some(ref hostname) = self.localhost {
            hostname.clone()
        } else {
            if infer_from_host {
                let subdomain = slug::slugify(&self.host);
                format!("{}.{}", subdomain, HOST_DEV)
            } else {
                HOST.to_string()
            }
        }
    }

    pub fn is_syntax_enabled(&self, name: &ProfileName) -> bool {
        if let Some(ref syntax) = self.syntax {
            return syntax.is_enabled(name);
        }
        false
    }

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

                if let Some(fluent) = cfg.fluent.as_mut() {
                    fluent.fallback = Some(cfg.lang.to_string());
                    fluent.fallback_id = fluent.fallback.as_ref().unwrap().parse()?;
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

                if let Some(db) = cfg.db.as_mut() {
                    if let Some(collators) = db.load.as_mut() {
                        for (_, v) in collators {
                            if let Some(ref from) = v.from {
                                if from.is_relative() {
                                    let mut tmp = build.source.clone();
                                    tmp.push(from);
                                    v.from = Some(tmp);
                                }
                            }
                        }
                    }
                }

                if let Some(search) = cfg.search.as_mut() {
                    search.prepare();
                }
                if let Some(feed) = cfg.feed.as_mut() {
                    feed.prepare();
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

        // Better error message when looking in the cwd
        if pth == PathBuf::from("") {
            if let Some(cwd) = resolve_cwd() {
                pth = cwd;
            }
        }

        let target_pth = pth.clone();

        //println!("Path {}", pth.display());

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

        Err(Error::NoSiteConfig(target_pth))
    }

    pub fn get_project(&self) -> PathBuf {
        self.project.as_ref().unwrap().clone()
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
    pub shared: Option<String>,
    #[serde(skip)]
    pub fallback: Option<String>,
    #[serde(skip)]
    pub fallback_id: LanguageIdentifier,
}

impl Default for FluentConfig {
    fn default() -> Self {
        Self {
            fallback: None,
            shared: Some(String::from(CORE_FTL)),
            fallback_id: String::from(LANG).parse().unwrap(),
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
#[serde(default)]
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
pub struct SeriesConfig {
    pub pages: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MinifyConfig {
    pub html: Option<MinifyFormat>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MinifyFormat {
    pub profiles: Vec<ProfileName>,
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
