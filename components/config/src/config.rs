use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::path::{Path, PathBuf};

use url::Url;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use log::debug;
use unic_langid::LanguageIdentifier;

use crate::{
    date::DateConfig,
    dependency::{DependencyDefinitionMap, DependencyMap},
    engine::TemplateEngine,
    feed::FeedConfig,
    fluent::FluentConfig,
    hook::{HookConfig, HookMap},
    indexer::DataBase,
    link::LinkConfig,
    live_reload::LiveReload,
    menu::MenuConfig,
    minify::MinifyConfig,
    page::{Author, Page},
    plugin::Plugin,
    profile::{NodeConfig, ProfileName, ProfileSettings, Profiles},
    redirect::RedirectConfig,
    repository::RepositoryConfig,
    robots::RobotsConfig,
    script::ScriptAsset,
    search::SearchConfig,
    sitemap::SiteMapConfig,
    style::StyleAsset,
    sync::SyncConfig,
    syntax::SyntaxConfig,
    tags::{link::LinkTag, script::ScriptTag},
    test::TestConfig,
    transform::TransformConfig,
    utils::href::UrlPath,
    Error,
};

pub const SITE: &str = "site";
pub const BUILD: &str = "build";
pub const RELEASE: &str = "release";
pub const SITE_LOCK: &str = "site.lock";
pub const SITE_TOML: &str = "site.toml";
pub const LOCALES: &str = "locales";
pub const MAIN_FTL: &str = "main.ftl";
pub const LANG_KEY: &str = "lang";
pub const HOST_KEY: &str = "host";
pub const FLUENT_KEY: &str = "fluent";
pub const FALLBACK_KEY: &str = "fallback";
pub const SHARED_KEY: &str = "shared";
pub const REDIRECT_KEY: &str = "redirect";
pub const PACKAGE: &str = "package";
pub const HTML: &str = "html";
pub const INDEX_STEM: &str = "index";
pub const INDEX_HTML: &str = "index.html";
pub const ERROR_HTML: &str = "404.html";
pub const MD: &str = "md";
pub const TOML: &str = "toml";
pub const JSON: &str = "json";
pub const ASSETS: &str = "assets";
pub const STYLES: &str = "styles";
pub const SCRIPTS: &str = "scripts";
pub const FONTS: &str = "fonts";
pub const PLUGINS: &str = "plugins";
pub const PARTIALS: &str = "partials";
pub const LAYOUTS: &str = "layouts";
pub const INCLUDES: &str = "includes";
pub const COLLECTIONS: &str = "collections";
pub const RESOURCES: &str = "resources";
pub const LANG: &str = "en";
pub const CHARSET: &str = "utf-8";
pub const TAGS: &str = "tags";

/// Used when multiple virtual hosts and inferring
/// a sub-domain from the primary host name.
pub const HOST_DEV: &str = "loopback.space";
pub const LOCALHOST: &str = "localhost";
pub const LOOPBACK_IP: &str = "127.0.0.1";

pub const ADDR: &str = "0.0.0.0";
pub const HOST: &str = "localhost";
pub const PORT: u16 = 8888;
pub const PORT_SSL: u16 = 8843;

pub const PORT_DOCS: u16 = 9988;
pub const PORT_DOCS_SSL: u16 = 9943;

pub const SCHEME_HTTPS: &str = "https:";
pub const SCHEME_HTTP: &str = "http:";
pub const SCHEME_DATA: &str = "data:";
pub const SCHEME_DELIMITER: &str = "//";
pub const SCHEME_WSS: &str = "wss:";
pub const SCHEME_WS: &str = "ws:";

pub const SCHEME_FILE: &str = "file:";
pub const SCHEME_TAR_LZMA: &str = "tar+xz:";
pub const SCHEME_PLUGIN: &str = "plugin:";

pub const PLUGIN: &str = "plugin.toml";
pub const PLUGIN_NS: &str = "::";
pub const PLUGIN_SPEC: &str = "@";
pub const PLUGIN_BLUEPRINT_NAMESPACE: &str = "std::blueprint";
pub const LATEST: &str = "latest";
pub const PACKAGE_NAME: &str = "package.tar.xz";

/// Prefix applied when extracting packages from archives.
pub const PLUGIN_ARCHIVE_PREFIX: &str = "pkg";

// For open graph defaults.
pub const OG_TYPE: &str = "type";
pub const OG_WEBSITE: &str = "website";
pub const OG_URL: &str = "url";
pub const OG_IMAGE: &str = "image";
pub const OG_TITLE: &str = "title";
pub const OG_DESCRIPTION: &str = "description";

pub const LAYOUT_HBS: &str = "main.hbs";
pub const MAIN: &str = "main";
//pub const MAIN_CSS: &str = "main.css";
//pub const MAIN_JS: &str = "main.js";
pub const DEFAULT_LAYOUT_NAME: &str = "std::core::main";

const DEFAULT_STYLE: &str = "assets/styles/main.css";
const DEFAULT_SCRIPT: &str = "assets/scripts/main.js";

const DEFAULT_ICON: &str = "favicon.ico";
const DEFAULT_ICON_DATA: &str =
    "data:image/gif;base64,R0lGODlhEAAQAAAAACwAAAAAAQABAAACASgAOw==";

pub const DEFAULT_PWA_MANIFEST: &str = "app.webmanifest";

pub const PUBLIC_HTML: &str = "public_html";

// Environment variables
pub const ENV_DISABLE_SSL:&str = "UWE_DISABLE_SSL";
pub const ENV_LOOPBACK_HOST:&str = "UWE_LOOPBACK_HOST";
pub const ENV_WEBDAV:&str = "UWE_WEBDAV";

// Webdav
pub const WEBDAV_MOUNT_PATH: &str = "/-/webdav";

const fn default_engine() -> TemplateEngine {
    TemplateEngine::Handlebars
}

pub const DEFAULT_ENGINE: TemplateEngine = default_engine();

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
    lang: String,
    host: String,

    /// Project version
    #[serde_as(as = "DisplayFromStr")]
    version: Version,

    charset: Option<String>,
    repository: Option<RepositoryConfig>,
    engine: Option<TemplateEngine>,
    icon: Option<UrlPath>,
    manifest: Option<UrlPath>,

    // Host name when running locally which overrides the inferred
    // localhost subdomain
    pub local_domain: Option<String>,

    pub build: Option<ProfileSettings>,
    pub workspace: Option<WorkspaceConfig>,
    pub fluent: FluentConfig,
    node: NodeConfig,
    pub page: Option<Page>,
    pub pages: Option<HashMap<String, Page>>,
    redirects: RedirectConfig,
    pub date: Option<DateConfig>,
    pub link: Option<LinkConfig>,
    pub profile: Option<HashMap<String, ProfileSettings>>,
    pub publish: Option<PublishConfig>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    authors: HashMap<String, Author>,

    #[serde(skip_serializing_if = "HashSet::is_empty")]
    hook: HashSet<HookConfig>,
    #[serde(skip)]
    hook_map: Option<HookMap>,

    // Menus keyed by name
    pub menu: Option<MenuConfig>,

    // Optional sitemap config
    sitemap: SiteMapConfig,

    // Optional robots config
    robots: RobotsConfig,

    dependencies: Option<DependencyDefinitionMap>,
    dependencies_map: Option<DependencyMap>,

    syntax: Option<SyntaxConfig>,
    pub transform: Option<TransformConfig>,
    pub search: Option<SearchConfig>,
    pub feed: Option<FeedConfig>,

    style: Option<StyleAsset>,
    script: Option<ScriptAsset>,

    pub db: Option<DataBase>,

    sync: Option<SyncConfig>,

    pub minify: Option<MinifyConfig>,
    live_reload: Option<LiveReload>,

    // Commit digest when available
    commit: Option<String>,

    test: TestConfig,

    #[serde(skip)]
    file: PathBuf,

    #[serde(skip)]
    project: PathBuf,

    // Name injected when this config is a workspace member
    #[serde(skip)]
    member_name: Option<String>,

    // Map of URLs for workspace members
    #[serde(skip)]
    member_urls: Option<HashMap<String, String>>,

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
            manifest: None,
            charset: Some(String::from(CHARSET)),
            repository: None,
            engine: Some(Default::default()),
            local_domain: None,
            build: Some(Default::default()),
            workspace: None,
            fluent: Default::default(),
            hook: Default::default(),
            hook_map: None,
            node: Default::default(),
            page: Some(Default::default()),
            pages: None,
            redirects: Default::default(),
            date: Some(Default::default()),
            link: Some(Default::default()),
            profile: Some(Default::default()),
            publish: Some(Default::default()),
            authors: HashMap::new(),
            menu: None,
            sitemap: Default::default(),
            robots: Default::default(),
            dependencies: None,
            dependencies_map: None,
            syntax: None,
            transform: Some(Default::default()),
            search: None,
            feed: None,
            style: None,
            script: None,
            db: None,
            sync: Some(Default::default()),
            minify: None,
            live_reload: Some(Default::default()),

            project: PathBuf::from(""),
            file: PathBuf::from(""),

            commit: None,
            test: Default::default(),
            member_name: None,
            member_urls: None,
        }
    }
}

impl Config {
    pub fn hooks(&self) -> &Option<HookMap> {
        &self.hook_map
    }

    pub fn hooks_mut(&mut self) -> &mut Option<HookMap> {
        &mut self.hook_map
    }

    pub fn syntax(&self) -> &Option<SyntaxConfig> {
        &self.syntax
    }

    pub fn fluent(&self) -> &FluentConfig {
        &self.fluent
    }

    pub fn test(&self) -> &TestConfig {
        &self.test
    }

    pub fn robots(&self) -> &RobotsConfig {
        &self.robots
    }

    pub fn redirects(&self) -> &RedirectConfig {
        &self.redirects
    }

    pub fn node(&self) -> &NodeConfig {
        &self.node
    }

    pub fn sitemap(&self) -> &SiteMapConfig {
        &self.sitemap
    }

    pub fn dependencies(&self) -> &Option<DependencyMap> {
        &self.dependencies_map
    }

    pub fn member_name(&self) -> &Option<String> {
        &self.member_name
    }

    pub fn set_member_name(&mut self, name: &str) {
        self.member_name = Some(name.to_owned());
    }

    pub fn member_urls(&self) -> &Option<HashMap<String, String>> {
        &self.member_urls
    }

    pub fn set_member_urls(&mut self, urls: HashMap<String, String>) {
        self.member_urls = Some(urls);
    }

    pub fn commit(&self) -> &Option<String> {
        &self.commit
    }

    pub fn manifest(&self) -> &Option<UrlPath> {
        &self.manifest
    }

    pub fn set_commit(&mut self, commit: Option<String>) {
        self.commit = commit;
    }

    pub fn live_reload(&self) -> &LiveReload {
        self.live_reload.as_ref().unwrap()
    }

    pub fn sync(&self) -> &SyncConfig {
        self.sync.as_ref().unwrap()
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

    pub fn file(&self) -> &PathBuf {
        &self.file
    }

    pub fn engine(&self) -> &TemplateEngine {
        self.engine.as_ref().unwrap_or(&DEFAULT_ENGINE)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn lang(&self) -> &str {
        &self.lang
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
        if let Some(domain) = self.local_domain.clone() {
            domain
        } else {
            if infer_from_host {
                self.dev_local_host_name(&self.host)
            } else {
                HOST.to_string()
            }
        }
    }

    pub fn dev_local_host_name(&self, host: &str) -> String {
        let subdomain = slug::slugify(host);
        format!("{}.{}", subdomain, HOST_DEV)
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
        //if let Some(base) = file.parent() {
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
            cfg.file = path.to_path_buf();

            // Ensure that lang is a valid identifier
            let lang_id = parse_language(&cfg.lang)?;

            // Ensure the host is a valid Url
            parse_host(&cfg.host)?;

            if let Some(deps) = cfg.dependencies.take() {
                let mut dependency_map: DependencyMap = deps.try_into()?;

                // To ease the development of blueprints we include the
                // plugin dependencies too so they don't have to be duplicated
                if let Some(parent) = file.parent() {
                    let plugin_file = parent.join(PLUGIN);
                    if plugin_file.exists() {
                        let content = utils::fs::read_string(plugin_file)?;
                        let plugin: Plugin = toml::from_str(&content)?;
                        dependency_map.append(plugin.dependencies().clone());
                    }
                }

                cfg.dependencies_map = Some(dependency_map);
            }

            cfg.fluent.prepare(lang_id);

            if !cfg.hook.is_empty() {
                let exec_hooks: HashSet<HookConfig> =
                    cfg.hook.drain().collect();
                cfg.hook_map = Some(HookMap::from(exec_hooks));
            }

            if let Some(date) = cfg.date.as_mut() {
                date.prepare();
            }
            if let Some(db) = cfg.db.as_mut() {
                db.prepare()?;
            }
            if let Some(search) = cfg.search.as_mut() {
                search.prepare();
            }
            if let Some(feed) = cfg.feed.as_mut() {
                feed.prepare();
            }
            for (k, v) in cfg.authors.iter_mut() {
                v.alias.get_or_insert(k.to_string());
            }
            if let Some(menu) = cfg.menu.as_mut() {
                menu.prepare();
            }

            return Ok(cfg);
        }
        //}
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
    pub members: Vec<String>,
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
#[serde(rename_all = "kebab-case")]
pub struct AwsPublishEnvironment {
    pub prefix: Option<String>,
    pub bucket: Option<String>,
    keep_remote: Option<bool>,
}

impl AwsPublishEnvironment {
    pub fn keep_remote(&self) -> bool {
        self.keep_remote.is_some() && self.keep_remote.unwrap()
    }
}
