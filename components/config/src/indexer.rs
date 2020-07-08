use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// This is the configuration option for generating
// an index, it is exposed here so that we can use
// easily in the main configuration as well as those
// for data sources.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SourceType {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "toml")]
    Toml,
}

impl Default for SourceType {
    fn default() -> Self {
        SourceType::Toml
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SourceProvider {
    #[serde(rename = "documents")]
    Documents,
    #[serde(rename = "pages")]
    Pages,
}

impl Default for SourceProvider {
    fn default() -> Self {
        SourceProvider::Pages
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DataSource {
    #[serde(rename = "type")]
    pub kind: Option<SourceType>,
    pub provider: Option<SourceProvider>,
    pub from: Option<PathBuf>,
    #[serde(alias = "on")]
    pub index: Option<HashMap<String, IndexRequest>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct IndexRequest {
    pub key: Option<String>,
}
