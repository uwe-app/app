use std::collections::HashMap;
use std::path::{Path, PathBuf};

use url::Url;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use log::debug;
use unic_langid::LanguageIdentifier;

use crate::{
    date::DateConfig,
    dependency::DependencyMap,
    engine::TemplateEngine,
    feed::FeedConfig,
    fluent::FluentConfig,
    hook::HookMap,
    indexer::{DataBase, IndexRequest},
    layout::LayoutConfig,
    link::LinkConfig,
    live_reload::LiveReload,
    menu::MenuConfig,
    minify::MinifyConfig,
    page::{Author, Page},
    powered::Powered,
    profile::{ProfileName, ProfileSettings, Profiles},
    redirect::RedirectConfig,
    repository::RepositoryConfig,
    robots::RobotsConfig,
    script::ScriptAsset,
    search::SearchConfig,
    sitemap::SiteMapConfig,
    style::StyleAsset,
    syntax::SyntaxConfig,
    tags::{link::LinkTag, script::ScriptTag},
    transform::TransformConfig,
    utils::href::UrlPath,
    Error,
};

pub static SITE: &str = "site";
pub static BUILD: &str = "build";
pub static RELEASE: &str = "release";
pub static SITE_LOCK: &str = "site.lock";
pub static SITE_TOML: &str = "site.toml";
pub static LOCALES: &str = "locales";
pub static MAIN_FTL: &str = "main.ftl";
pub static LANG_KEY: &str = "lang";
pub static HOST_KEY: &str = "host";
pub static FLUENT_KEY: &str = "fluent";
pub static FALLBACK_KEY: &str = "fallback";
pub static SHARED_KEY: &str = "shared";
pub static REDIRECT_KEY: &str = "redirect";
pub static PACKAGE: &str = "package";
pub static HTML: &str = "html";
pub static INDEX_STEM: &str = "index";
pub static INDEX_HTML: &str = "index.html";
pub static MD: &str = "md";
pub static TOML: &str = "toml";
pub static JSON: &str = "json";
pub static ASSETS: &str = "assets";
pub static STYLES: &str = "styles";
pub static SCRIPTS: &str = "scripts";
pub static FONTS: &str = "fonts";
pub static PLUGINS: &str = "plugins";
pub static PARTIALS: &str = "partials";
pub static LAYOUTS: &str = "layouts";
pub static INCLUDES: &str = "includes";
pub static DATASOURCES: &str = "data-sources";
pub static RESOURCES: &str = "resources";
pub static LANG: &str = "en";
pub static CHARSET: &str = "utf-8";
pub static TAGS: &str = "tags";

/// Used when multiple virtual hosts and inferring
/// a sub-domain from the primary host name.
pub static HOST_DEV: &str = "loopback.space";

pub static ADDR: &str = "127.0.0.1";
pub static HOST: &str = "localhost";
pub static PORT: u16 = 8888;
pub static PORT_SSL: u16 = 8843;

pub static PORT_DOCS: u16 = 9988;
pub static PORT_DOCS_SSL: u16 = 9943;

pub static SCHEME_HTTPS: &str = "https:";
pub static SCHEME_HTTP: &str = "http:";
pub static SCHEME_DATA: &str = "data:";
pub static SCHEME_DELIMITER: &str = "//";
pub static SCHEME_WSS: &str = "wss:";
pub static SCHEME_WS: &str = "ws:";

pub static SCHEME_FILE: &str = "file:";
pub static SCHEME_TAR_LZMA: &str = "tar+xz:";
pub static SCHEME_PLUGIN: &str = "plugin:";

pub static PLUGIN: &str = "plugin.toml";
pub static PLUGIN_NS: &str = "::";

/// Prefix applied when extracting packages from archives.
pub static PLUGIN_ARCHIVE_PREFIX: &str = "pkg";

// For open graph defaults.
pub static OG_TYPE: &str = "type";
pub static OG_WEBSITE: &str = "website";
pub static OG_URL: &str = "url";
pub static OG_IMAGE: &str = "image";
pub static OG_TITLE: &str = "title";
pub static OG_DESCRIPTION: &str = "description";

pub static LAYOUT_HBS: &str = "main.hbs";
pub static MAIN: &str = "main";
//pub static MAIN_CSS: &str = "main.css";
//pub static MAIN_JS: &str = "main.js";
pub static DEFAULT_LAYOUT_NAME: &str = "std::core::main";

static DEFAULT_STYLE: &str = "assets/styles/main.css";
static DEFAULT_SCRIPT: &str = "assets/scripts/main.js";

static DEFAULT_ICON: &str = "assets/favicon.png";
static DEFAULT_ICON_DATA: &str =
    "data:image/gif;base64,R0lGODlhEAAQAAAAACwAAAAAAQABAAACASgAOw==";

const fn default_engine() -> TemplateEngine {
    TemplateEngine::Handlebars
}

pub static DEFAULT_ENGINE: TemplateEngine = default_engine();

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

pub fn parse_language<S: AsRef<str>>(
    lang: S,
) -> Result<LanguageIdentifier, Error> {
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

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub lang: String,
    pub host: String,

    /// Plugin version.
    #[serde_as(as = "DisplayFromStr")]
    version: Version,
    charset: Option<String>,
    repository: Option<RepositoryConfig>,
    engine: Option<TemplateEngine>,
    icon: Option<UrlPath>,
    powered: Option<Powered>,

    // Host name when running locally which overrides the inferred
    // localhost subdomain
    pub localhost: Option<String>,

    pub build: Option<ProfileSettings>,
    pub workspace: Option<WorkspaceConfig>,
    pub fluent: Option<FluentConfig>,
    pub hooks: Option<HookMap>,
    pub node: Option<NodeConfig>,
    pub page: Option<Page>,
    pub pages: Option<HashMap<String, Page>>,
    pub redirect: Option<RedirectConfig>,
    pub date: Option<DateConfig>,
    pub link: Option<LinkConfig>,
    pub profile: Option<HashMap<String, ProfileSettings>>,
    pub publish: Option<PublishConfig>,
    pub index: Option<HashMap<String, IndexRequest>>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    authors: HashMap<String, Author>,

    // Menus keyed by name
    pub menu: Option<MenuConfig>,

    // Optional sitemap config
    pub sitemap: Option<SiteMapConfig>,

    // Optional robots config
    pub robots: Option<RobotsConfig>,

    pub dependencies: Option<DependencyMap>,

    pub layout: Option<LayoutConfig>,

    pub syntax: Option<SyntaxConfig>,
    pub transform: Option<TransformConfig>,
    pub search: Option<SearchConfig>,
    pub feed: Option<FeedConfig>,

    style: Option<StyleAsset>,
    script: Option<ScriptAsset>,

    pub db: Option<DataBase>,

    pub minify: Option<MinifyConfig>,
    pub livereload: Option<LiveReload>,

    // Commit digest when available
    commit: Option<String>,

    #[serde(skip)]
    pub file: Option<PathBuf>,

    #[serde(skip)]
    pub project: PathBuf,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(skip_deserializing)]
    website: Url,

}

impl Default for Config {
    fn default() -> Self {
        Config {
            lang: String::from(LANG),
            host: String::from(HOST),
            version: Version::from((1, 0, 0)),
            website: format!("{}{}:{}", SCHEME_HTTP, HOST, PORT)
                .parse()
                .unwrap(),

            icon: None,
            powered: Some(Default::default()),
            charset: Some(String::from(CHARSET)),
            repository: None,
            engine: Some(Default::default()),
            localhost: None,
            build: Some(Default::default()),
            workspace: None,
            fluent: Some(Default::default()),
            hooks: Some(Default::default()),
            node: Some(Default::default()),
            page: Some(Default::default()),
            pages: None,
            redirect: None,
            date: Some(Default::default()),
            link: Some(Default::default()),
            profile: Some(Default::default()),
            publish: Some(Default::default()),
            index: None,
            authors: HashMap::new(),
            menu: None,
            sitemap: Some(Default::default()),
            robots: Some(Default::default()),
            dependencies: None,
            layout: None,
            syntax: None,
            transform: Some(Default::default()),
            search: None,
            feed: None,
            style: None,
            script: None,
            db: None,
            minify: None,
            livereload: Some(Default::default()),

            file: None,
            project: PathBuf::from(""),

            commit: None,
        }
    }
}

impl Config {

    pub fn commit(&self) -> &Option<String> {
        &self.commit
    }

    pub fn set_commit(&mut self, commit: Option<String>) {
        self.commit = commit;
    }

    pub fn powered(&self) -> Option<&Powered> {
        self.powered.as_ref()
    }

    pub fn default_script() -> ScriptAsset {
        ScriptAsset::Tag(ScriptTag::new(DEFAULT_SCRIPT.to_string()))
    }

    pub fn default_style() -> LinkTag {
        LinkTag::new_style_sheet(DEFAULT_STYLE.to_string(), None)
    }

    pub fn default_icon() -> LinkTag {
        LinkTag::new_icon(DEFAULT_ICON_DATA.to_string())
    }

    pub fn default_icon_url() -> &'static str {
        DEFAULT_ICON
    }

    pub fn project(&self) -> &PathBuf {
        &self.project
    }

    pub fn engine(&self) -> &TemplateEngine {
        self.engine.as_ref().unwrap_or(&DEFAULT_ENGINE)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn icon_mut(&mut self) -> &mut Option<UrlPath> {
        &mut self.icon
    }

    pub fn style_mut(&mut self) -> &mut Option<StyleAsset> {
        &mut self.style
    }

    pub fn script_mut(&mut self) -> &mut Option<ScriptAsset> {
        &mut self.script
    }

    pub fn website(&self) -> &Url {
        &self.website
    }

    pub fn set_website(&mut self, url: Url) {
        self.website = url;
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn charset(&self) -> &str {
        self.charset.as_ref().unwrap()
    }

    pub fn repository(&self) -> &Option<RepositoryConfig> {
        &self.repository
    }

    pub fn authors(&self) -> &HashMap<String, Author> {
        &self.authors
    }

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
            return syntax.profiles().is_match(name);
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

                let project = resolve_project(&file);
                if project.is_none() {
                    return Err(Error::ProjectResolve(file.to_path_buf()));
                }

                cfg.project = project.unwrap();

                // Must be a canonical path
                let path = file.canonicalize()?;
                cfg.file = Some(path.to_path_buf());

                // Ensure that lang is a valid identifier
                let lang_id = parse_language(&cfg.lang)?;

                // Ensure the host is a valid Url
                parse_host(&cfg.host)?;

                // Ensure source and target paths are relative
                // to the base
                let mut build = cfg.build.as_mut().unwrap();
                if build.source.is_relative() {
                    build.source = base.to_path_buf().join(&build.source);
                }
                if build.target.is_relative() {
                    build.target = base.to_path_buf().join(&build.target);
                }

                if let Some(fluent) = cfg.fluent.as_mut() {
                    fluent.prepare(&cfg.lang, lang_id);
                }
                if let Some(date) = cfg.date.as_mut() {
                    date.prepare();
                }
                if let Some(db) = cfg.db.as_mut() {
                    db.prepare(&build.source);
                }
                if let Some(search) = cfg.search.as_mut() {
                    search.prepare();
                }
                if let Some(feed) = cfg.feed.as_mut() {
                    feed.prepare();
                }
                if let Some(link) = cfg.link.as_mut() {
                    link.prepare(&build.source)?;
                }
                for (k, v) in cfg.authors.iter_mut() {
                    v.alias.get_or_insert(k.to_string());
                }
                if let Some(menu) = cfg.menu.as_mut() {
                    menu.prepare();
                }

                return Ok(cfg);
            }
        }
        return Ok(Default::default());
    }

    pub fn load<P: AsRef<Path>>(
        source: P,
        walk_ancestors: bool,
    ) -> Result<Self, Error> {
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceConfig {
    pub members: Vec<PathBuf>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct NodeConfig {
    // Allow custom mappings for NODE_ENV
    pub debug: Option<String>,
    pub release: Option<String>,
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
