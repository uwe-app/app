use std::collections::HashMap;
use std::path::PathBuf;

use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};

pub static SEARCH_JS: &str = "search.js";
pub static SEARCH_WASM: &str = "search.wasm";

static ID: &str = "site-index";
static INDEX: &str = "/search.idx";
static JS: &str = "/search.js";
static WASM: &str = "/search.wasm";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct SearchConfig {
    // Copy the `search.js` and `search.wasm` files to the URL paths
    // referenced by `js` and `wasm`
    pub bundle: Option<bool>,
    // The URL relative to the site root for the javascript file
    pub js: Option<String>,
    // The URL relative to the site root for the wasm file
    pub wasm: Option<String>,

    // Search index configurations
    #[serde(flatten)]
    pub items: HashMap<String, SearchItemConfig>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            bundle: Some(true),
            js: Some(JS.to_string()),
            wasm: Some(WASM.to_string()),
            items: HashMap::new(),
        }
    }
}

impl SearchConfig {
    // Prepare the configuration by assigning id fields
    // and compiling the glob matchers
    pub fn prepare(&mut self) {
        for (k, v) in self.items.iter_mut() {
            v.id = Some(k.to_string());
            v.include_match = v.includes.iter().map(|g| g.compile_matcher()).collect();
            v.exclude_match = v.excludes.iter().map(|g| g.compile_matcher()).collect();
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct SearchItemConfig {
    // The URL path for the search index file relative to the site root
    pub index: Option<String>,

    // Maximum number of results displayed for a query
    pub results: Option<u8>,

    // Configuration options for indexing behavior
    pub includes: Vec<Glob>,
    pub excludes: Vec<Glob>,

    // The identifier used when registering the search widget
    #[serde(skip)]
    pub id: Option<String>,

    // Number of excerpts to buffer
    #[serde(skip)]
    pub excerpt_buffer: Option<u8>,

    // Maximum number of excerpts per result
    #[serde(skip)]
    pub excerpts_per_result: Option<u8>,

    #[serde(skip)]
    pub include_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub exclude_match: Vec<GlobMatcher>,
}

impl Default for SearchItemConfig {
    fn default() -> Self {
        Self {
            id: Some(ID.to_string()),
            index: Some(INDEX.to_string()),
            results: Some(10),
            excerpt_buffer: Some(8),
            excerpts_per_result: Some(5),
            includes: Vec::new(),
            excludes: Vec::new(),
            include_match: Vec::new(),
            exclude_match: Vec::new(),
        }
    }
}

impl SearchItemConfig {
    pub fn filter(&self, href: &str) -> bool {
        for glob in self.exclude_match.iter() {
            if glob.is_match(href) { return false; }
        }
        if self.include_match.is_empty() { return true; }
        for glob in self.include_match.iter() {
            if glob.is_match(href) { return true; }
        }
        false
    }

    pub fn get_output_path(&self, base: &PathBuf) -> PathBuf {
        let val = self.index.as_ref().unwrap().trim_start_matches("/");
        return base.join(utils::url::to_path_separator(val));
    }
}
