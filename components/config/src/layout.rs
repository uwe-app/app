use std::collections::hash_map;
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct LayoutConfig {
    #[serde(flatten)]
    pub layouts: HashMap<String, PathBuf>,
}

impl LayoutConfig {
    pub fn iter(&self) -> hash_map::Iter<'_, String, PathBuf> {
        self.layouts.iter()
    }
}
