use std::cmp::Ordering;
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use serde_json::Value;
use serde_with::skip_serializing_none;

static DEFAULT_PARAMETER: &str = "result";

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
pub struct PageInfo {
    pub size: usize,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(default, rename_all = "kebab-case")]
pub struct IndexQuery {
    pub name: String,
    pub index: String,
    pub parameter: Option<String>,
    pub include_docs: Option<bool>,
    pub each: Option<bool>,
    pub keys: Option<bool>,
    pub key_type: Option<KeyType>,
    pub unique: Option<bool>,
    pub desc: Option<bool>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub page: Option<PageInfo>,
    pub sort: Option<String>,
}

impl Default for IndexQuery {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            index: "".to_string(),
            parameter: None,
            include_docs: None,
            each: None,
            keys: None,
            key_type: Some(Default::default()),
            unique: None,
            desc: None,
            offset: Some(0),
            limit: None,
            page: None,
            sort: None,
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
    pub fn to_vec(&self) -> Vec<IndexQuery> {
        match self {
            QueryList::One(query) => vec![query.clone()],
            QueryList::Many(items) => items.to_vec(),
        } 
    }

    pub fn to_assign_vec(&self) -> Vec<IndexQuery> {
        self.to_vec()
            .iter()
            .filter(|q| (q.each.is_none() || (q.each.is_some() && !q.each.unwrap())) && q.page.is_none())
            .map(IndexQuery::clone)
            .collect::<Vec<_>>()
    }

    pub fn to_each_vec(&self) -> Vec<IndexQuery> {
        self.to_vec()
            .iter()
            .filter(|q| q.each.is_some() && q.each.unwrap())
            .map(IndexQuery::clone)
            .collect::<Vec<_>>()
    }

    pub fn to_page_vec(&self) -> Vec<IndexQuery> {
        self.to_vec()
            .iter()
            .filter(|q| q.page.is_some())
            .map(IndexQuery::clone)
            .collect::<Vec<_>>()
    }
}

#[derive(Eq, Debug, Serialize, Deserialize, Clone, Default)]
pub struct IndexKey {
    pub name: String,
    pub value: Value,
}

impl Ord for IndexKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name) 
    }
}

impl PartialOrd for IndexKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for IndexKey {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name 
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum QueryValue {
    One(Value),
    Many(Vec<Value>),
}

impl Default for QueryValue {
    fn default() -> Self {
        QueryValue::Many(vec![])
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum KeyResult {
    Full(IndexKey),
    Name(String),
    Value(Value),
}

impl Default for KeyResult {
    fn default() -> Self {
        KeyResult::Name("".to_string())
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QueryResult {
    pub id: Option<String>,
    pub key: Option<KeyResult>,
    pub value: Option<QueryValue>,
}
