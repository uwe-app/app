use super::StemmingConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type Fields = HashMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct File {
    pub title: String,
    pub url: String,
    #[serde(flatten)]
    pub source: DataSource,

    pub id: Option<String>,
    #[serde(default)]
    pub stemming_override: Option<StemmingConfig>,

    #[serde(flatten, default)]
    pub fields: Fields,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DataSource {
    #[serde(rename = "contents")]
    Contents(String),

    #[serde(rename = "path")]
    FilePath(String),
}

impl Default for DataSource {
    fn default() -> Self {
        DataSource::Contents(String::default())
    }
}
