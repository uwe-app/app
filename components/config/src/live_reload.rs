use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub static LIVERELOAD_FILE: &str = "__livereload.js";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LiveReload {
    pub notify: Option<bool>,
    // This is undocumented but here if it must be used
    pub file: Option<PathBuf>,
}

impl Default for LiveReload {
    fn default() -> Self {
        Self {
            notify: Some(true),
            file: Some(PathBuf::from(LIVERELOAD_FILE)),
        }
    }
}
