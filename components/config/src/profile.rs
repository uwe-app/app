use std::collections::HashMap;
use std::convert::From;
use std::fmt;
use std::mem;
use std::path::PathBuf;

use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;

use url::Url;

use super::config::{self, Config};
use super::matcher::GlobPatternMatcher;
use super::robots::RobotsConfig;
use super::server::TlsConfig;
use super::sitemap::SiteMapConfig;

static DEBUG: &str = "debug";
static RELEASE: &str = "release";

static DEVELOPMENT: &str = "development";
static PRODUCTION: &str = "production";

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(from = "String", untagged)]
pub enum ProfileName {
    Debug,
    Release,
    Custom(String),
}

impl Serialize for ProfileName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ProfileName::Debug => serializer.serialize_str(DEBUG),
            ProfileName::Release => serializer.serialize_str(RELEASE),
            ProfileName::Custom(ref val) => serializer.serialize_str(val),
        }
    }
}

impl Default for ProfileName {
    fn default() -> Self {
        ProfileName::Debug
    }
}

impl From<String> for ProfileName {
    fn from(s: String) -> Self {
        if s == DEBUG {
            ProfileName::Debug
        } else if s == RELEASE {
            ProfileName::Release
        } else {
            ProfileName::Custom(s)
        }
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ProfileName::Custom(ref val) => write!(f, "{}", val),
            ProfileName::Debug => write!(f, "{}", DEBUG),
            ProfileName::Release => write!(f, "{}", RELEASE),
        }
    }
}

impl ProfileName {
    pub fn get_node_env(
        &self,
        debug: Option<String>,
        release: Option<String>,
    ) -> String {
        match self {
            ProfileName::Debug => {
                if let Some(env) = debug {
                    return env;
                }
                return DEVELOPMENT.to_string();
            }
            ProfileName::Release => {
                if let Some(env) = release {
                    return env;
                }
                return PRODUCTION.to_string();
            }
            ProfileName::Custom(s) => return s.to_string(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct ProfileSettings {
    #[serde(skip)]
    pub name: ProfileName,

    pub source: PathBuf,
    pub target: PathBuf,

    pub types: Option<RenderTypes>,
    pub strict: Option<bool>,
    pub parallel: Option<bool>,

    pub assets: Option<PathBuf>,
    pub locales: Option<PathBuf>,
    pub includes: Option<PathBuf>,
    pub partials: Option<PathBuf>,
    pub data_sources: Option<PathBuf>,
    pub layout: Option<PathBuf>,

    pub extend: Option<Vec<String>>,

    pub profile: Option<String>,
    pub live: Option<bool>,
    pub release: Option<bool>,

    pub short_codes: Option<bool>,
    pub use_layout: Option<bool>,

    pub rewrite_index: Option<bool>,
    pub include_index: Option<bool>,

    pub incremental: Option<bool>,
    pub pristine: Option<bool>,
    pub force: Option<bool>,
    // Collate page data when defined
    pub collate: Option<bool>,

    pub write_redirects: Option<bool>,

    // Base URL to strip when building links etc
    pub base: Option<String>,

    // Specific set of paths to build
    pub paths: Option<Vec<PathBuf>>,

    // A base URL to strip from links
    pub base_href: Option<String>,

    pub host: Option<String>,
    pub port: Option<u16>,
    pub scheme: Option<String>,
    pub tls: Option<TlsConfig>,

    pub robots: Option<RobotsConfig>,
    pub sitemap: Option<SiteMapConfig>,
    pub resources: Option<Resources>,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            name: Default::default(),

            source: PathBuf::from(config::SITE),
            target: PathBuf::from(config::BUILD),
            types: Some(Default::default()),
            strict: None,
            parallel: None,

            assets: Some(PathBuf::from(config::ASSETS)),
            locales: Some(PathBuf::from(config::LOCALES)),
            includes: Some(PathBuf::from(config::INCLUDES)),
            partials: Some(PathBuf::from(config::PARTIALS)),
            data_sources: Some(PathBuf::from(config::DATASOURCES)),
            layout: Some(PathBuf::from(config::LAYOUT_HBS)),

            rewrite_index: None,
            extend: None,
            short_codes: None,

            profile: None,

            // FIXME: use ServeConfig
            host: Some(config::HOST.to_string()),
            port: Some(config::PORT),
            scheme: Some(config::SCHEME_HTTPS.to_string()),
            tls: None,

            live: None,
            release: None,
            include_index: None,
            incremental: None,
            pristine: None,
            force: None,
            collate: None,
            write_redirects: None,
            base: None,
            paths: None,
            base_href: None,

            use_layout: Some(true),

            robots: None,
            sitemap: None,
            resources: None,
        }
    }
}

impl ProfileSettings {
    pub fn new_release() -> Self {
        let mut settings: ProfileSettings = Default::default();
        settings.release = Some(true);
        settings
    }

    pub fn append(&mut self, other: &mut Self) {
        self.source = mem::take(&mut other.source);
        self.target = mem::take(&mut other.target);

        if other.types.is_some() {
            self.types = mem::take(&mut other.types)
        }
        if other.strict.is_some() {
            self.strict = mem::take(&mut other.strict)
        }
        if other.parallel.is_some() {
            self.parallel = mem::take(&mut other.parallel)
        }

        if other.assets.is_some() {
            self.assets = mem::take(&mut other.assets)
        }
        if other.locales.is_some() {
            self.locales = mem::take(&mut other.locales)
        }
        if other.includes.is_some() {
            self.includes = mem::take(&mut other.includes)
        }
        if other.partials.is_some() {
            self.partials = mem::take(&mut other.partials)
        }
        if other.data_sources.is_some() {
            self.data_sources = mem::take(&mut other.data_sources)
        }
        if other.layout.is_some() {
            self.layout = mem::take(&mut other.layout)
        }

        if other.rewrite_index.is_some() {
            self.rewrite_index = mem::take(&mut other.rewrite_index)
        }
        if other.extend.is_some() {
            self.extend = mem::take(&mut other.extend)
        }
        if other.short_codes.is_some() {
            self.short_codes = mem::take(&mut other.short_codes)
        }

        if other.host.is_some() {
            self.host = mem::take(&mut other.host)
        }
        if other.port.is_some() {
            self.port = mem::take(&mut other.port)
        }
        if other.scheme.is_some() {
            self.scheme = mem::take(&mut other.scheme)
        }
        if other.tls.is_some() {
            self.tls = mem::take(&mut other.tls)
        }

        if other.live.is_some() {
            self.live = mem::take(&mut other.live)
        }
        if other.release.is_some() {
            self.release = mem::take(&mut other.release)
        }
        if other.include_index.is_some() {
            self.include_index = mem::take(&mut other.include_index)
        }
        if other.incremental.is_some() {
            self.incremental = mem::take(&mut other.incremental)
        }
        if other.pristine.is_some() {
            self.pristine = mem::take(&mut other.pristine)
        }
        if other.force.is_some() {
            self.force = mem::take(&mut other.force)
        }
        if other.collate.is_some() {
            self.collate = mem::take(&mut other.collate)
        }
        if other.write_redirects.is_some() {
            self.write_redirects = mem::take(&mut other.write_redirects)
        }
        if other.base.is_some() {
            self.base = mem::take(&mut other.base)
        }
        if other.paths.is_some() {
            self.paths = mem::take(&mut other.paths)
        }
        if other.base_href.is_some() {
            self.base_href = mem::take(&mut other.base_href)
        }

        if other.use_layout.is_some() {
            self.use_layout = mem::take(&mut other.use_layout)
        }

        if other.robots.is_some() {
            self.robots = mem::take(&mut other.robots)
        }
        if other.sitemap.is_some() {
            self.sitemap = mem::take(&mut other.sitemap)
        }
        if other.resources.is_some() {
            self.resources = mem::take(&mut other.resources)
        }
    }

    pub fn get_canonical_url(
        &self,
        conf: &config::Config,
    ) -> crate::Result<Url> {
        if self.is_release() {
            let scheme = self.scheme.as_ref().unwrap();
            Ok(Url::parse(&crate::to_url_string(scheme, &conf.host, None))?)
        } else {
            Ok(Url::parse(&crate::to_url_string(
                config::SCHEME_HTTP,
                self.host.as_ref().unwrap(),
                self.port.clone(),
            ))?)
        }
    }

    pub fn get_host_url(&self, conf: &config::Config) -> String {
        // FIXME: do not unwrap here, return the Result?
        self.get_canonical_url(conf).unwrap().to_string()
    }

    pub fn set_defaults(&mut self) {
        if let None = self.strict {
            self.strict = Some(true);
        }
        if let None = self.parallel {
            self.parallel = Some(true);
        }
        if let None = self.pristine {
            self.pristine = Some(true);
        }
        if let None = self.incremental {
            self.incremental = Some(false);
        }
        if let None = self.collate {
            self.collate = Some(true);
        }
    }

    pub fn get_host(&self) -> String {
        if let Some(ref host) = self.host {
            host.clone()
        } else {
            config::HOST.to_string()
        }
    }

    pub fn get_port(&self) -> u16 {
        if let Some(ref port) = self.port {
            port.clone()
        } else {
            config::PORT
        }
    }

    pub fn is_parallel(&self) -> bool {
        self.parallel.is_some() && self.parallel.unwrap()
    }

    pub fn is_live(&self) -> bool {
        self.live.is_some() && self.live.unwrap()
    }

    pub fn is_release(&self) -> bool {
        self.release.is_some() && self.release.unwrap()
    }

    pub fn is_force(&self) -> bool {
        self.force.is_some() && self.force.unwrap()
    }

    pub fn is_incremental(&self) -> bool {
        self.incremental.is_some() && self.incremental.unwrap()
    }

    pub fn is_pristine(&self) -> bool {
        self.pristine.is_some() && self.pristine.unwrap()
    }

    pub fn should_collate(&self) -> bool {
        self.collate.is_some() && self.collate.unwrap()
    }

    pub fn should_use_layout(&self) -> bool {
        self.use_layout.is_some() && self.use_layout.unwrap()
    }

    pub fn should_use_short_codes(&self) -> bool {
        self.short_codes.is_some() && self.short_codes.unwrap()
    }

    pub fn should_include_index(&self) -> bool {
        self.include_index.is_some() && self.include_index.unwrap()
    }

    pub fn should_rewrite_index(&self) -> bool {
        self.rewrite_index.is_some() && self.rewrite_index.unwrap()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeOptions {
    // Project root
    pub project: PathBuf,
    // Root for the input source files
    pub source: PathBuf,
    // Root of the output
    pub output: PathBuf,
    // Target output directory including a build tag
    pub base: PathBuf,
    // Target output directory including a build tag and
    // a locale identifier when multilingual
    //pub target: PathBuf,
    // The computed profile to use
    pub settings: ProfileSettings,
}

impl RuntimeOptions {
    pub fn get_canonical_url(
        &self,
        config: &Config,
        include_lang: Option<&str>,
    ) -> crate::Result<Url> {
        let mut base = self.settings.get_canonical_url(config)?;
        // FIXME: RESTORE
        //if self.locales.multi {
            //if let Some(lang) = include_lang {
                //base = base.join(lang)?;
            //}
        //}
        Ok(base)
    }

    pub fn get_layout_path(&self) -> PathBuf {
        self.source.join(self.settings.layout.as_ref().unwrap())
    }

    pub fn get_assets_path(&self) -> PathBuf {
        self.source.join(self.settings.assets.as_ref().unwrap())
    }

    pub fn get_includes_path(&self) -> PathBuf {
        self.source.join(self.settings.includes.as_ref().unwrap())
    }

    pub fn get_partials_path(&self) -> PathBuf {
        self.source.join(self.settings.partials.as_ref().unwrap())
    }

    pub fn get_data_sources_path(&self) -> PathBuf {
        self.source
            .join(self.settings.data_sources.as_ref().unwrap())
    }

    pub fn get_locales(&self) -> PathBuf {
        self.source.join(self.settings.locales.as_ref().unwrap())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenderTypes {
    #[serde(flatten)]
    pub types: HashMap<String, PageType>,
}

impl Default for RenderTypes {
    fn default() -> Self {
        let mut types: HashMap<String, PageType> = HashMap::new();
        types.insert(
            config::MD.to_string(),
            PageType {
                markdown: Some(true),
                map: Some(config::HTML.to_string()),
            },
        );
        Self { types }
    }
}

impl RenderTypes {
    // Get list of file extensions to render
    pub fn render(&self) -> Vec<String> {
        self.types.keys().map(|v| v.to_string()).collect::<Vec<_>>()
    }

    // Get the extension mapping
    pub fn map(&self) -> HashMap<String, String> {
        let mut map: HashMap<String, String> = HashMap::new();
        for (k, v) in self.types.iter() {
            if let Some(ref ext) = v.map {
                map.insert(k.to_string(), ext.to_string());
            }
        }
        map
    }

    // Get list of extension to parse as markdown
    pub fn markdown(&self) -> Vec<String> {
        self.types
            .iter()
            .filter(|(_k, v)| v.markdown.is_some() && v.markdown.unwrap())
            .map(|(k, _v)| k.to_string())
            .collect::<Vec<_>>()
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PageType {
    // Map this page type to another extension
    pub map: Option<String>,
    // Parse this page type as markdown
    pub markdown: Option<bool>,
}

impl Default for PageType {
    fn default() -> Self {
        Self {
            map: None,
            markdown: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Resources {
    /// Ignore these resources.
    ///
    /// Typically used for files created outside the program
    /// that should just be mapped as links.
    pub ignore: ResourceGroup,
    pub symlink: ResourceGroup,
    pub copy: ResourceGroup,
}

impl Resources {
    pub fn prepare(&mut self) {
        self.ignore.matcher.compile();
        self.symlink.matcher.compile();
        self.copy.matcher.compile();
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ResourceGroup {
    #[serde(flatten)]
    pub matcher: GlobPatternMatcher,
}
