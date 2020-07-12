use std::fmt;
use std::path::PathBuf;
use std::convert::From;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;

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
pub struct BuildArguments {
    pub max_depth: Option<usize>,
    pub profile: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub live: Option<bool>,
    pub release: Option<bool>,
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
}

impl Default for BuildArguments {
    fn default() -> Self {
        Self {
            max_depth: None,
            profile: None,
            host: None,
            port: None,
            live: None,
            release: None,
            include_index: None,
            incremental: Some(false),
            pristine: Some(true),
            force: None,
            write_redirects: None,
            base: None,
            layout: None,
            paths: None,
        }
    }
}

