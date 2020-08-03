use std::path::PathBuf;

use serde::{Deserialize, Serialize};

static OUTPUT:&str = "search.idx";
static ID:&str = "site-index";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SearchConfig {
    pub id: Option<String>,
    pub output: Option<PathBuf>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            id: Some(ID.to_string()),
            output: Some(PathBuf::from(OUTPUT)),
        }
    }
}
