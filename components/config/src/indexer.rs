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
    // Use the id as the value; primarily useful for the 
    // default all index
    pub identity: Option<bool>,
    // Group on the key value, key must point to a string
    // or array of strings
    pub group: Option<bool>,
    // The document key to use for the index, may be dot-delimited
    // to specify a path to the value
    pub key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum KeyType {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "value")]
    Value,
}

impl Default for KeyType {
    fn default() -> Self {
        KeyType::Full
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct IndexQuery {
    pub name: String,
    pub index: String,
    pub parameter: Option<String>,
    pub include_docs: Option<bool>,
    pub each: Option<bool>,
    pub keys: Option<KeyType>,
    pub values: Option<bool>,
    pub unique: Option<bool>,
    pub desc: Option<bool>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl Default for IndexQuery {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            index: ALL_INDEX.to_string(),
            parameter: None,
            include_docs: Some(false),
            each: Some(false),
            keys: None,
            values: Some(false),
            unique: Some(false),
            desc: Some(false),
            offset: Some(0),
            limit: None,
        }
    }
}

impl IndexQuery {
    pub fn is_unique(&self) -> bool {
        return self.unique.is_some() && self.unique.unwrap();
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

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
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
