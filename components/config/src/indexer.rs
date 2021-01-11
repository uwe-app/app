use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{to_value, Value};
use serde_with::skip_serializing_none;

use globset::Glob;

use crate::{Error, Result};

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
    #[serde(rename = "files")]
    Files,
    #[serde(rename = "pages")]
    Pages,
}

impl Default for SourceProvider {
    fn default() -> Self {
        SourceProvider::Pages
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DataBase {
    pub load: Option<HashMap<String, DataSource>>,
}

impl DataBase {
    pub(crate) fn prepare(&self) -> Result<()> {
        if let Some(ref collators) = self.load {
            for (_, v) in collators {
                if let Some(ref from) = v.from {
                    if from.is_absolute() {
                        return Err(Error::FromAbsolute(from.to_path_buf()));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DataSource {
    #[serde(rename = "type")]
    pub kind: Option<SourceType>,
    pub provider: Option<SourceProvider>,
    pub from: Option<PathBuf>,
    // Omit files that match this pattern when building
    // the index; patterns are matched relative to the containing
    // directory.
    pub excludes: Vec<Glob>,
    #[serde(alias = "on")]
    pub index: Option<HashMap<String, IndexRequest>>,
}

impl Default for DataSource {
    fn default() -> Self {
        Self {
            kind: Some(Default::default()),
            provider: Some(Default::default()),
            from: None,
            index: Some(HashMap::new()),
            excludes: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct IndexRequest {
    // The document key to use for the index, may be dot-delimited
    // to specify a path to the value. If the special `identity` value
    // is specified then the index is sorted by the generated document id.
    pub key: String,
    // List of filters to use when building the index.
    //pub filters: Option<HashMap<String, bool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum KeyType {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "id")]
    Id,
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
pub struct GroupBy {
    pub path: String,
    pub expand: Option<bool>,
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
    pub group: Option<GroupBy>,
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
            group: None,
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
            .filter(|q| {
                (q.each.is_none() || (q.each.is_some() && !q.each.unwrap()))
                    && q.page.is_none()
            })
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
    pub id: String,
    pub name: String,
    pub doc_id: String,
    pub sort: String,
    pub value: Value,
}

impl Ord for IndexKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort.cmp(&other.sort)
    }
}

impl PartialOrd for IndexKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for IndexKey {
    fn eq(&self, other: &Self) -> bool {
        self.sort == other.sort
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

impl QueryResult {
    pub fn to_value(&self, query: &IndexQuery) -> Result<Value> {
        // When only keys are requested transpose so we don't have
        // the unnecessary `key` field name.
        let keys = query.keys.is_some() && query.keys.unwrap();
        if keys {
            if let Some(ref key) = self.key {
                match key {
                    KeyResult::Name(ref name) => {
                        return Ok(Value::String(name.clone()))
                    }
                    KeyResult::Value(val) => return Ok(val.clone()),
                    KeyResult::Full(ref key_val) => {
                        return Ok(to_value(key_val.clone())?)
                    }
                }
            }
        }

        Ok(to_value(self)?)
    }
}
