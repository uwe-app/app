use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

static ALL_INDEX: &str = "all";
static DEFAULT_PARAMETER: &str = "documents";
static DEFAULT_VALUE_PARAMETER: &str = "value";

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

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct IndexQuery {
    pub name: String,
    pub index: String,
    pub parameter: Option<String>,
    pub include_docs: Option<bool>,
    pub each: Option<bool>,
    pub keys: Option<bool>,
    pub values: Option<bool>,
    pub flat: Option<bool>,
}

impl IndexQuery {
    pub fn is_flat(&self) -> bool {
        return self.index == ALL_INDEX.to_string() || self.flat.is_some() && self.flat.unwrap();
    }

    pub fn get_parameter(&self) -> String {
        if let Some(param) = &self.parameter {
            return param.clone();
        }
        return DEFAULT_PARAMETER.to_string();
    }

    pub fn get_value_parameter(&self) -> String {
        let each = self.each.is_some() && self.each.unwrap();
        if each {
            if let Some(param) = &self.parameter {
                return param.clone();
            }
        }
        return DEFAULT_VALUE_PARAMETER.to_string();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum QueryList {
    One(IndexQuery),
    Many(Vec<IndexQuery>),
}

impl Default for QueryList {
    fn default() -> Self {
        QueryList::One(Default::default())
    }
}

impl QueryList {
    pub fn to_vec(self) -> Vec<IndexQuery> {
        match self {
            QueryList::One(query) => vec![query.clone()],
            QueryList::Many(items) => items.to_vec(),
        } 
    }
}
