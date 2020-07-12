use std::path::PathBuf;

use config::ProfileName;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

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

    // Rewrite output destinations to a directory
    // with an index.html file
    pub rewrite_index: bool,

    pub max_depth: Option<usize>,
    pub release: bool,
    pub tag: ProfileName,
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
