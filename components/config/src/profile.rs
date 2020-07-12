use std::fmt;
use std::path::PathBuf;
use std::convert::From;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;

use super::config;

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
    where S: Serializer {
        match *self {
            ProfileName::Debug => {
                serializer.serialize_str(DEBUG)
            },
            ProfileName::Release => {
                serializer.serialize_str(RELEASE)
            },
            ProfileName::Custom(ref val) => {
                serializer.serialize_str(val)
            },
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
    pub fn get_node_env(&self, debug: Option<String>, release: Option<String>) -> String {
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
    pub strict: Option<bool>,
    pub pages: Option<PathBuf>,
    pub assets: Option<PathBuf>,
    pub includes: Option<PathBuf>,
    pub partials: Option<PathBuf>,
    pub data_sources: Option<PathBuf>,
    pub resources: Option<PathBuf>,

    //pub rewrite_index: Option<bool>,
    pub follow_links: Option<bool>,
    pub render: Option<Vec<String>>,

    pub max_depth: Option<usize>,
    pub profile: Option<String>,
    pub live: Option<bool>,
    pub release: Option<bool>,

    pub use_layout: Option<bool>,

    pub rewrite_index: Option<bool>,
    pub include_index: Option<bool>,

    pub incremental: Option<bool>,
    pub pristine: Option<bool>,
    pub force: Option<bool>,

    pub write_redirects: Option<bool>,

    // Base URL to strip when building links etc
    pub base: Option<String>,

    // Specific layout to use
    pub layout: Option<PathBuf>,

    // Specific set of paths to build
    pub paths: Option<Vec<PathBuf>>,

    // A base URL to strip from links
    pub base_href: Option<String>,

    pub host: Option<String>,
    pub port: Option<u16>,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            name: Default::default(),

            source: PathBuf::from(config::SITE),
            target: PathBuf::from(config::BUILD),
            strict: Some(true),
            pages: Some(PathBuf::from(config::PAGE_DATA)),
            assets: Some(PathBuf::from(config::ASSETS)),
            includes: Some(PathBuf::from(config::INCLUDES)),
            partials: Some(PathBuf::from(config::PARTIALS)),
            data_sources: Some(PathBuf::from(config::DATASOURCES)),
            resources: Some(PathBuf::from(config::RESOURCES)),
            rewrite_index: None,
            follow_links: Some(true),
            render: None,

            max_depth: None,
            profile: None,
            host: Some(config::HOST.to_string()),
            port: Some(config::PORT),
            live: None,
            release: None,
            include_index: None,
            incremental: Some(false),
            pristine: Some(true),
            force: None,
            write_redirects: None,
            base: None,
            layout: Some(PathBuf::from(config::LAYOUT_HBS)),
            paths: None,
            use_layout: Some(true),
            base_href: None,
        }
    }
}

impl ProfileSettings {

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

    pub fn should_use_layout(&self) -> bool {
        self.use_layout.is_some() && self.use_layout.unwrap()
    }

    pub fn should_include_index(&self) -> bool {
        self.include_index.is_some() && self.include_index.unwrap()
    }

    pub fn should_rewrite_index(&self) -> bool {
        self.rewrite_index.is_some() && self.rewrite_index.unwrap()
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RuntimeOptions {
    // Root of the input
    pub source: PathBuf,
    // Root of the output
    pub output: PathBuf,
    // Target output directory including a build tag
    pub base: PathBuf,
    // Target output directory including a build tag and
    // a locale identifier when multilingual
    pub target: PathBuf,
    // The computed profile to use
    pub settings: ProfileSettings,
}
