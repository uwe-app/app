use std::path::PathBuf;

use serde::{Deserialize, Serialize};

static ID:&str = "site-index";
static OUTPUT:&str = "search.idx";
static JS:&str = "/search.js";
static WASM:&str = "/search.wasm";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SearchConfig {
    // The identifier used when registering the search widget
    pub id: Option<String>,
    // The output file for the search index
    pub output: Option<PathBuf>,
    // The URL relative to the site root for the javascript file
    pub js: Option<String>,
    // The URL relative to the site root for the wasm file
    pub wasm: Option<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            id: Some(ID.to_string()),
            output: Some(PathBuf::from(OUTPUT)),
            js: Some(JS.to_string()),
            wasm: Some(WASM.to_string()),
        }
    }
}
