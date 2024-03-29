use std::collections::HashMap;
use std::convert::From;
use std::convert::Infallible;
use std::fmt;
use std::mem;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;

use url::Url;

use crate::{
    config::{self, Config},
    server::SslConfig,
    utils::matcher::GlobPatternMatcher,
};

const DEBUG: &str = "debug";
const RELEASE: &str = "release";
const DIST: &str = "dist";
const TEST: &str = "test";

const DEVELOPMENT: &str = "development";
const PRODUCTION: &str = "production";

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeConfig {
    #[serde(flatten)]
    map: HashMap<ProfileName, String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert(ProfileName::Debug, DEVELOPMENT.to_string());
        map.insert(ProfileName::Release, PRODUCTION.to_string());
        map.insert(ProfileName::Dist, PRODUCTION.to_string());
        map.insert(ProfileName::Test, TEST.to_string());
        Self { map }
    }
}

/// Trait for settings that maintain a list of profiles.
///
/// Typically this is used to indicate that the settings
/// should only apply to the specified profiles.
pub trait Profiles {
    fn profiles(&self) -> &ProfileFilter;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ProfileFilter {
    Flag(bool),
    Name(ProfileName),
    List(Vec<ProfileName>),
}

impl Default for ProfileFilter {
    fn default() -> Self {
        ProfileFilter::Flag(true)
    }
}

impl ProfileFilter {
    pub fn is_match(&self, name: &ProfileName) -> bool {
        match *self {
            ProfileFilter::Flag(enabled) => enabled,
            ProfileFilter::Name(ref target) => target == name,
            ProfileFilter::List(ref target) => target.contains(name),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(from = "String", untagged)]
pub enum ProfileName {
    Debug,
    Release,
    Dist,
    Test,
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
            ProfileName::Dist => serializer.serialize_str(DIST),
            ProfileName::Test => serializer.serialize_str(TEST),
            ProfileName::Custom(ref val) => serializer.serialize_str(val),
        }
    }
}

impl Default for ProfileName {
    fn default() -> Self {
        ProfileName::Release
    }
}

impl From<String> for ProfileName {
    fn from(s: String) -> Self {
        if s == DEBUG {
            ProfileName::Debug
        } else if s == RELEASE {
            ProfileName::Release
        } else if s == DIST {
            ProfileName::Dist
        } else if s == TEST {
            ProfileName::Test
        } else {
            ProfileName::Custom(s)
        }
    }
}

impl FromStr for ProfileName {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(ProfileName::Debug),
            "release" => Ok(ProfileName::Release),
            "dist" => Ok(ProfileName::Dist),
            "test" => Ok(ProfileName::Test),
            _ => Ok(ProfileName::Custom(s.to_owned())),
        }
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", {
            match *self {
                ProfileName::Custom(ref val) => val,
                ProfileName::Debug => DEBUG,
                ProfileName::Release => RELEASE,
                ProfileName::Dist => DIST,
                ProfileName::Test => TEST,
            }
        })
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct ProfileSettings {
    pub name: ProfileName,

    pub source: PathBuf,
    pub target: PathBuf,

    /// Allow command execution
    pub exec: Option<bool>,

    /// Include drafts
    pub include_drafts: Option<bool>,

    pub types: Option<RenderTypes>,
    pub strict: Option<bool>,
    pub parallel: Option<bool>,
    pub offline: Option<bool>,

    pub assets: Option<PathBuf>,
    pub locales: Option<PathBuf>,
    pub includes: Option<PathBuf>,
    pub partials: Option<PathBuf>,
    pub layouts: Option<PathBuf>,
    pub collections: Option<PathBuf>,

    pub extend: Option<Vec<String>>,

    pub live: Option<bool>,
    pub launch: Option<String>,
    pub release: Option<bool>,

    // Name for the default layout, when not specified
    // and `layouts/main.hbs` exists it will be used
    // otherwise the default layout name `std::core::main`
    // will be used instead.
    pub layout: Option<String>,

    pub rewrite_index: Option<bool>,
    pub include_index: Option<bool>,
    /// Should we pass the commit hash to page templates.
    pub include_commit: Option<bool>,

    pub incremental: Option<bool>,
    pub pristine: Option<bool>,
    pub force: Option<bool>,

    pub write_redirect_files: Option<bool>,

    // Base URL to strip when building links etc
    pub base: Option<String>,

    // Specific set of paths to build
    pub paths: Option<Vec<PathBuf>>,

    // A base URL to strip from links
    pub base_href: Option<String>,

    pub host: Option<String>,
    pub port: Option<u16>,
    pub scheme: Option<String>,
    pub tls: Option<SslConfig>,

    pub resources: Option<Resources>,

    /// List of workspace members to filter.
    pub member: Vec<String>,
}

impl From<&ProfileName> for ProfileSettings {
    fn from(name: &ProfileName) -> Self {
        let mut settings = ProfileSettings {
            name: name.clone(),
            ..Default::default()
        };

        match name {
            ProfileName::Release => {
                settings.release = Some(true);
            }
            ProfileName::Dist => {
                settings.include_index = Some(true);
            }
            _ => {}
        }

        settings
    }
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            name: Default::default(),
            source: PathBuf::from(config::SITE),
            target: PathBuf::from(config::BUILD),
            exec: None,
            include_drafts: None,
            types: Some(Default::default()),
            strict: None,
            parallel: None,
            offline: None,

            assets: Some(PathBuf::from(config::ASSETS)),
            locales: Some(PathBuf::from(config::LOCALES)),
            includes: Some(PathBuf::from(config::INCLUDES)),
            partials: Some(PathBuf::from(config::PARTIALS)),
            layouts: Some(PathBuf::from(config::LAYOUTS)),
            collections: Some(PathBuf::from(config::COLLECTIONS)),

            rewrite_index: Some(true),
            extend: None,

            // FIXME: use ServeConfig
            host: Some(config::HOST.to_string()),
            port: Some(config::PORT),
            scheme: Some(config::SCHEME_HTTPS.to_string()),
            tls: None,

            live: None,
            launch: None,
            release: None,
            layout: None,
            include_index: None,
            incremental: None,
            pristine: None,
            force: None,
            write_redirect_files: None,
            base: None,
            paths: None,
            base_href: None,

            resources: None,
            member: Vec::new(),

            include_commit: None,
        }
    }
}

impl ProfileSettings {
    pub fn get_node_env(&self, config: &NodeConfig) -> String {
        if let Some(ref name) = config.map.get(&self.name) {
            name.to_string()
        } else {
            PRODUCTION.to_string()
        }
    }

    /// Determine if this build profile can execute hooks.
    pub fn can_exec(&self) -> bool {
        self.exec.is_some() && self.exec.unwrap()
    }

    pub fn include_commit(&self) -> bool {
        self.include_commit.is_some() && self.include_commit.unwrap()
    }

    pub fn write_redirect_files(&self) -> bool {
        self.write_redirect_files.is_some()
            && self.write_redirect_files.unwrap()
    }

    /// Determine if drafts should be included.
    pub fn include_drafts(&self) -> bool {
        self.include_drafts.is_some() && self.include_drafts.unwrap()
    }

    pub fn append(&mut self, other: &mut Self) {
        self.name = mem::take(&mut other.name);

        self.source = mem::take(&mut other.source);
        self.target = mem::take(&mut other.target);

        // NOTE: Do not inherit `exec` otherwise it
        // NOTE: defeats the point of `--exec` as authors
        // NOTE: could just add:
        //
        // NOTE: [build]
        // NOTE: exec = true
        //
        // NOTE: Which would bypass the test for explicit
        // NOTE: execution capability granted on the command line.

        if other.include_drafts.is_some() {
            self.include_drafts = mem::take(&mut other.include_drafts)
        }
        if other.types.is_some() {
            self.types = mem::take(&mut other.types)
        }
        if other.strict.is_some() {
            self.strict = mem::take(&mut other.strict)
        }
        if other.parallel.is_some() {
            self.parallel = mem::take(&mut other.parallel)
        }
        if other.offline.is_some() {
            self.offline = mem::take(&mut other.offline)
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
        if other.layouts.is_some() {
            self.layouts = mem::take(&mut other.layouts)
        }
        if other.collections.is_some() {
            self.collections = mem::take(&mut other.collections)
        }

        if other.rewrite_index.is_some() {
            self.rewrite_index = mem::take(&mut other.rewrite_index)
        }
        if other.extend.is_some() {
            self.extend = mem::take(&mut other.extend)
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
        if other.layout.is_some() {
            self.layout = mem::take(&mut other.layout)
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
        if other.write_redirect_files.is_some() {
            self.write_redirect_files =
                mem::take(&mut other.write_redirect_files)
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

        if other.resources.is_some() {
            self.resources = mem::take(&mut other.resources)
        }

        self.member = mem::take(&mut other.member);

        if other.include_commit.is_some() {
            self.include_commit = mem::take(&mut other.include_commit);
        }
    }

    pub fn get_canonical_url(
        &self,
        conf: &Config,
        host: Option<&str>,
    ) -> crate::Result<Url> {
        if self.is_release() {
            let scheme = self.scheme.as_ref().unwrap();
            Ok(Url::parse(&crate::to_url_string(
                scheme,
                conf.host(),
                None,
            ))?)
        } else {
            let scheme = if self.tls.is_some() {
                config::SCHEME_HTTPS
            } else {
                config::SCHEME_HTTP
            };

            let port = self.get_canonical_port();

            Ok(Url::parse(&crate::to_url_string(
                scheme,
                host.unwrap_or(self.host.as_ref().unwrap()),
                port,
            ))?)
        }
    }

    pub fn get_host_url(
        &self,
        conf: &config::Config,
        host: Option<&str>,
    ) -> crate::Result<Url> {
        Ok(self.get_canonical_url(conf, host)?)
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

    pub fn get_canonical_port(&self) -> u16 {
        if let Some(ref tls) = self.tls {
            tls.port()
        } else {
            self.get_port()
        }
    }

    pub fn is_offline(&self) -> bool {
        self.offline.is_some() && self.offline.unwrap()
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

    pub fn should_include_index(&self) -> bool {
        self.include_index.is_some() && self.include_index.unwrap()
    }

    pub fn should_rewrite_index(&self) -> bool {
        self.rewrite_index.is_some() && self.rewrite_index.unwrap()
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
