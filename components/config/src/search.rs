use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub static SEARCH_JS:&str = "search.js";
pub static SEARCH_WASM:&str = "search.wasm";

static ID:&str = "site-index";
static OUTPUT:&str = "search.idx";
static JS:&str = "/assets/js/search.js";
static WASM:&str = "/assets/wasm/search.wasm";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct SearchConfig {
    // The identifier used when registering the search widget
    pub id: Option<String>,
    // The output file for the search index
    pub output: Option<PathBuf>,
    // The URL relative to the site root for the javascript file
    pub js: Option<String>,
    // The URL relative to the site root for the wasm file
    pub wasm: Option<String>,
    // Copy the `search.js` and `search.wasm` files to the URL paths
    // referenced by `js` and `wasm`
    pub copy_runtime: Option<bool>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            id: Some(ID.to_string()),
            output: Some(PathBuf::from(OUTPUT)),
            js: Some(JS.to_string()),
            wasm: Some(WASM.to_string()),
            copy_runtime: Some(true),
        }
    }
}
