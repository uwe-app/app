use std::collections::BTreeMap;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SourceProvider {
    #[serde(rename = "documents")]
    Documents,
    #[serde(rename = "pages")]
    Pages,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataSource {
    #[serde(rename = "type")]
    pub kind: SourceType,
    pub provider: SourceProvider,
    pub index: Option<BTreeMap<String, IndexRequest>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexRequest {
    pub key: Option<String>,
}
