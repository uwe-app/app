use std::fmt;
use std::path::PathBuf;
use std::convert::From;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

static DEBUG: &str = "debug";
static RELEASE: &str = "release";

static DEVELOPMENT: &str = "development";
static PRODUCTION: &str = "production";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(from = "String")]
pub enum BuildTag {
    Custom(String),
    Debug,
    Release,
}

impl Default for BuildTag {
    fn default() -> Self {
        BuildTag::Debug
    }
}

impl From<String> for BuildTag {
    fn from(s: String) -> Self {
        if s == DEBUG {
            BuildTag::Debug
        } else if s == RELEASE {
            BuildTag::Release
        } else {
            BuildTag::Custom(s) 
        }
    }
}

impl fmt::Display for BuildTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BuildTag::Custom(ref val) => write!(f, "{}", val),
            BuildTag::Debug => write!(f, "{}", DEBUG),
            BuildTag::Release => write!(f, "{}", RELEASE),
        }
    }
}

impl BuildTag {
    pub fn get_node_env(&self, debug: Option<String>, release: Option<String>) -> String {
        match self {
            BuildTag::Debug => {
                if let Some(env) = debug {
                    return env;
                }
                return DEVELOPMENT.to_string();
            }
            BuildTag::Release => {
                if let Some(env) = release {
                    return env;
                }
                return PRODUCTION.to_string();
            }
            BuildTag::Custom(s) => return s.to_string(),
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

