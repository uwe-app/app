use std::collections::BTreeMap;
use std::fs::ReadDir;
use std::path::Path;
use std::path::PathBuf;

use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Map, Value};
use slug;
use thiserror::Error;

use config::page::Page;
use config::Config;
use utils;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Query should be array or object")]
    QueryType,

    #[error("Duplicate document id {id} ({name}.json)")]
    DuplicateId {id: String, name: String},

    #[error("The all index is reserved, choose another index name")]
    AllIndexReserved,

    #[error("Type error building index, keys must be string values")]
    IndexKeyType,

    #[error("No data source with name {0}")]
    NoDataSource(String),

    #[error("No index with name {0}")]
    NoIndex(String),

    #[error("No configuration {conf} for data source {key}")]
    NoDataSourceConf {conf: String, key: String},

    #[error("No {docs} directory for data source {key}")]
    NoDataSourceDocuments {docs: String, key: String},

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),
}

type Result<T> = std::result::Result<T, Error>;

static DATASOURCE_TOML: &str = "datasource.toml";
static DOCUMENTS: &str = "documents";
static ALL_INDEX: &str = "all";
static DEFAULT_PARAMETER: &str = "documents";
static DEFAULT_VALUE_PARAMETER: &str = "value";
static JSON: &str = "json";

pub fn get_query(data: &Page) -> Result<Vec<IndexQuery>> {
    //let generator_config = data.query;
    let mut page_generators: Vec<IndexQuery> = Vec::new();
    if let Some(cfg) = &data.query {
        // Single object declaration
        if cfg.is_object() {
            let conf = cfg.as_object().unwrap();
            let reference: IndexQuery = from_value(json!(conf))?;
            page_generators.push(reference);
        // Multiple array declaration
        } else if cfg.is_array() {
            let items = cfg.as_array().unwrap();
            for conf in items {
                let reference: IndexQuery = from_value(json!(conf))?;
                page_generators.push(reference);
            }
        } else {
            return Err(Error::QueryType);
        }
    }

    Ok(page_generators)
}

pub fn get_datasource_documents_path<P: AsRef<Path>>(source: P) -> PathBuf {
    let mut pth = source.as_ref().to_path_buf();
    pth.push(DOCUMENTS);
    pth
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataSourceConfig {
    pub index: Option<BTreeMap<String, IndexRequest>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexRequest {
    pub key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug)]
pub struct DataSource {
    pub site: PathBuf,
    pub source: PathBuf,
    pub config: DataSourceConfig,
    pub all: BTreeMap<String, Value>,
    pub indices: BTreeMap<String, ValueIndex>,
}

#[derive(Debug)]
pub struct ValueIndex {
    pub documents: BTreeMap<String, Vec<String>>,
}

impl ValueIndex {
    pub fn to_keys(&self) -> Vec<Value> {
        return self
            .documents
            .keys()
            .map(|k| {
                return json!(&k);
            })
            .collect::<Vec<_>>();
    }

    pub fn to_values(&self) -> Vec<Value> {
        return self
            .documents
            .values()
            .map(|v| {
                return json!(&v);
            })
            .collect::<Vec<_>>();
    }

    pub fn from_query(&self, query: &IndexQuery, docs: &BTreeMap<String, Value>) -> Vec<Value> {
        let include_docs = query.include_docs.is_some() && query.include_docs.unwrap();

        return self
            .documents
            .iter()
            .map(|(k, v)| {
                let id = slug::slugify(&k);
                let mut m = Map::new();

                m.insert("id".to_string(), json!(&id));
                m.insert("key".to_string(), json!(&k));

                if include_docs {
                    if query.is_flat() && v.len() == 1 {
                        let s = &v[0];
                        let mut d = Map::new();
                        d.insert("id".to_string(), json!(s));
                        if let Some(doc) = docs.get(s) {
                            d.insert("document".to_string(), json!(doc));
                        } else {
                            warn!("Query missing document for {}", s);
                        }
                        m.insert(query.get_value_parameter(), json!(&d));
                    } else {
                        let docs = v
                            .iter()
                            .map(|s| {
                                let mut m = Map::new();
                                if let Some(doc) = docs.get(s) {
                                    m.insert("id".to_string(), json!(s));
                                    m.insert("document".to_string(), json!(doc));
                                } else {
                                    warn!("Query missing document for {}", s);
                                }
                                m
                            })
                            .collect::<Vec<_>>();

                        m.insert(query.get_value_parameter(), json!(&docs));
                    }
                }

                json!(&m)
            })
            .collect::<Vec<_>>();
    }
}

impl DataSource {
    pub fn load(&mut self) -> Result<()> {
        let documents = get_datasource_documents_path(&self.source);
        let contents = documents.read_dir()?;
        for entry in contents {
            let path = entry?.path();
            if let Some(ext) = path.extension() {
                if ext == JSON {
                    let contents = utils::fs::read_string(&path)?;
                    let document: Value = serde_json::from_str(&contents)?;
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy().into_owned();
                        let id = slug::slugify(&name);
                        if self.all.contains_key(&id) {
                            return Err(Error::DuplicateId {id, name});
                        }
                        self.all.insert(id, document);
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DataSourceMap {
    pub map: BTreeMap<String, DataSource>,
}

impl DataSourceMap {
    pub fn new() -> Self {
        let map: BTreeMap<String, DataSource> = BTreeMap::new();
        DataSourceMap { map }
    }

    pub fn get_datasource_config_path<P: AsRef<Path>>(&self, source: P) -> PathBuf {
        let mut pth = source.as_ref().to_path_buf();
        pth.push(DATASOURCE_TOML);
        pth
    }

    pub fn load(&mut self, source: PathBuf, config: &Config) -> Result<()> {
        self.load_configurations(source, config)?;
        self.load_documents()?;
        self.configure_default_index()?;
        self.load_index()?;
        Ok(())
    }

    // Configure the default all index
    fn configure_default_index(&mut self) -> Result<()> {
        for (_, generator) in self.map.iter_mut() {
            if generator.config.index.is_none() {
                generator.config.index = Some(BTreeMap::new());
            }

            let index = generator.config.index.as_ref().unwrap();

            // Complain on reserved index name
            if index.contains_key(ALL_INDEX) {
                return Err(Error::AllIndexReserved);
            }

            if let Some(ref mut index) = generator.config.index.as_mut() {
                // Inherit key from index name
                for (k, v) in index.iter_mut() {
                    if v.key.is_none() {
                        v.key = Some(k.clone());
                    }
                }

                // Set up default all index
                index.insert(
                    ALL_INDEX.to_string(),
                    IndexRequest {
                        key: Some(ALL_INDEX.to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn load_index(&mut self) -> Result<()> {
        let type_err = Err(Error::IndexKeyType);

        for (_, generator) in self.map.iter_mut() {
            let index = generator.config.index.as_ref().unwrap();

            for (name, def) in index {
                let key = def.key.as_ref().unwrap();
                let mut values = ValueIndex {
                    documents: BTreeMap::new(),
                };

                for (id, document) in &generator.all {
                    if name == ALL_INDEX {
                        let items = values.documents.entry(id.clone()).or_insert(Vec::new());
                        items.push(id.clone());
                        continue;
                    }

                    if let Some(val) = document.get(&key) {
                        let mut candidates: Vec<&str> = Vec::new();

                        if !val.is_string() && !val.is_array() {
                            return type_err;
                        }

                        if let Some(s) = val.as_str() {
                            candidates.push(s);
                        }

                        if let Some(arr) = val.as_array() {
                            for val in arr {
                                if let Some(s) = val.as_str() {
                                    candidates.push(s);
                                } else {
                                    return type_err;
                                }
                            }
                        }

                        for s in candidates {
                            let items = values.documents.entry(s.to_string()).or_insert(Vec::new());
                            items.push(id.clone());
                        }
                    }
                }

                generator.indices.insert(name.clone(), values);
            }

            //for (k, idx) in &generator.indices {
            //println!("index key {:?}", k);
            //println!("{}", serde_json::to_string_pretty(&idx.to_keys()).unwrap());
            //}
        }
        Ok(())
    }

    pub fn query_index(&self, query: &IndexQuery) -> Result<Vec<Value>> {
        let name = &query.name;
        let idx_name = &query.index;
        let keys = query.keys.is_some() && query.keys.unwrap();
        let values = query.values.is_some() && query.values.unwrap();

        if let Some(generator) = self.map.get(name) {
            if let Some(idx) = generator.indices.get(idx_name) {
                if keys {
                    return Ok(idx.to_keys());
                } else if values {
                    return Ok(idx.to_values());
                }
                return Ok(idx.from_query(query, &generator.all));
            } else {
                return Err(Error::NoIndex(idx_name.to_string()));
            }
        } else {
            return Err(Error::NoDataSource(name.to_string()));
        }
        
    }

    fn load_documents(&mut self) -> Result<()> {
        for (k, g) in self.map.iter_mut() {
            info!("{} < {}", k, g.source.display());
            g.load()?;
        }
        Ok(())
    }

    fn load_config(&mut self, source: PathBuf, dir: ReadDir) -> Result<()> {
        for f in dir {
            let path = f?.path();
            if path.is_dir() {
                if let Some(nm) = path.file_name() {
                    let key = nm.to_string_lossy().into_owned();
                    let conf = self.get_datasource_config_path(&path);
                    if !conf.exists() || !conf.is_file() {
                        return Err(Error::NoDataSourceConf {
                            conf: DATASOURCE_TOML.to_string(),
                            key
                        });
                    }

                    let mut data = path.to_path_buf().clone();
                    data.push(DOCUMENTS);
                    if !data.exists() || !data.is_dir() {
                        return Err(Error::NoDataSourceDocuments {
                            docs: DOCUMENTS.to_string(),
                            key
                        });
                    }

                    let contents = utils::fs::read_string(conf)?;
                    let config: DataSourceConfig = toml::from_str(&contents)?;

                    let all: BTreeMap<String, Value> = BTreeMap::new();
                    let indices: BTreeMap<String, ValueIndex> = BTreeMap::new();

                    let generator = DataSource {
                        site: source.clone(),
                        source: path.to_path_buf(),
                        all,
                        indices,
                        config,
                    };

                    self.map.insert(key, generator);
                }
            }
        }
        Ok(())
    }

    fn load_configurations(&mut self, source: PathBuf, config: &Config) -> Result<()> {
        let src = config.get_datasources_path(&source);
        if src.exists() && src.is_dir() {
            let contents = src.read_dir()?;
            self.load_config(source, contents)?;
        }
        Ok(())
    }
}
