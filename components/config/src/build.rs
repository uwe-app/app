use std::fmt;
use std::convert::From;
use serde::{Deserialize, Serialize};

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

