use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl BuildTag {
    pub fn get_path_name(&self) -> String {
        match self {
            BuildTag::Debug => return "debug".to_string(),
            BuildTag::Release => return "release".to_string(),
            BuildTag::Custom(s) => return s.to_string(),
        }
    }

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

    //pub fn clone(&self) -> Self {
        //match self {
            //BuildTag::Debug => return BuildTag::Debug,
            //BuildTag::Release => return BuildTag::Release,
            //BuildTag::Custom(s) => return BuildTag::Custom(s.to_string()),
        //}
    //}
}

// FIXME: re-use the BuildArguments in the CompilerOptions!

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CompilerOptions {
    // Root of the input
    pub source: PathBuf,
    // Root of the output
    pub output: PathBuf,
    // Target output directory including a build tag
    pub base: PathBuf,
    // Target output directory including a build tag and
    // a locale identifier when multilingual
    pub target: PathBuf,

    pub clean_url: bool,

    pub max_depth: Option<usize>,
    pub release: bool,
    pub tag: BuildTag,
    pub live: bool,
    pub host: String,
    pub port: u16,
    pub force: bool,

    // Default layout file to use
    pub layout: PathBuf,

    // A base URL to strip from links
    pub base_href: Option<String>,

    // Specific paths to compile
    pub paths: Option<Vec<PathBuf>>,

    pub incremental: bool,

    pub include_index: bool,
}

