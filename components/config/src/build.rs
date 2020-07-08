use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BuildTag {
    Custom(String),
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "release")]
    Release,
}

impl Default for BuildTag {
    fn default() -> Self {
        BuildTag::Debug
    }
}

impl fmt::Display for BuildTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BuildTag::Custom(ref val) => write!(f, "{}", val),
            BuildTag::Debug => write!(f, "debug"),
            BuildTag::Release => write!(f, "release"),
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
                return "development".to_string();
            }
            BuildTag::Release => {
                if let Some(env) = release {
                    return env;
                }
                return "production".to_string();
            }
            BuildTag::Custom(s) => return s.to_string(),
        }
    }
}

