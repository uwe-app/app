use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub static SEARCH_JS:&str = "search.js";
pub static SEARCH_WASM:&str = "search.wasm";

static ID:&str = "site-index";
static TARGET:&str = "/search.idx";
static JS:&str = "/search.js";
static WASM:&str = "/search.wasm";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct SearchConfig {
    // The identifier used when registering the search widget
    pub id: Option<String>,
    // The URL path for the search index file relative to the site root
    pub target: Option<String>,
    // The URL relative to the site root for the javascript file
    pub js: Option<String>,
    // The URL relative to the site root for the wasm file
    pub wasm: Option<String>,
    // Copy the `search.js` and `search.wasm` files to the URL paths
    // referenced by `js` and `wasm`
    pub copy_runtime: Option<bool>,

    // Configuration options for indexing behavior
    pub index: Option<SearchIndexConfig>,

    // Maximum number of results displayed for a query
    pub results: Option<u8>,

    // Number of excerpts to buffer
    #[serde(skip_deserializing)]
    pub excerpt_buffer: Option<u8>,

    // Maximum number of excerpts per result
    #[serde(skip_deserializing)]
    pub excerpts_per_result: Option<u8>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            id: Some(ID.to_string()),
            target: Some(TARGET.to_string()),
            js: Some(JS.to_string()),
            wasm: Some(WASM.to_string()),
            copy_runtime: Some(true),
            index: Some(Default::default()),
            results: Some(10),
            excerpt_buffer: Some(8),
            excerpts_per_result: Some(5),
        }
    }
}

impl SearchConfig {
    pub fn get_output_path(&self, base: &PathBuf) -> PathBuf {
        let val = self.target.as_ref().unwrap().trim_start_matches("/");
        return base.join(utils::url::to_path_separator(val));
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default, rename_all = "kebab-case")]
pub struct SearchIndexConfig {
    pub filters: Vec<PathBuf>,
}
