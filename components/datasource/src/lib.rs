use std::sync::Arc;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::ReadDir;
use std::path::Path;
use std::path::PathBuf;

use log::{info, warn};
use serde::{Serialize, Deserialize};
use serde_json::{json, to_value, Map, Value};
use thiserror::Error;

use collator::CollateInfo;
use config::{Config, RuntimeOptions, FileInfo, FileOptions};
use config::indexer::{IndexQuery, KeyType, SourceProvider, DataSource as DataSourceConfig};

pub mod identifier;
pub mod provider;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Query should be array or object")]
    QueryType,

    #[error("Duplicate document id {key} ({path})")]
    DuplicateId {key: String, path: PathBuf},

    #[error("Type error building index, keys must be string values")]
    IndexKeyType,

    #[error("Data source document should be an object")]
    DataSourceDocumentNotAnObject,

    #[error("Data source document must have an id")]
    DataSourceDocumentNoId,

    #[error("No data source with name {0}")]
    NoDataSource(String),

    #[error("No index with name {0}")]
    NoIndex(String),

    #[error("No configuration {conf} for data source {key}")]
    NoDataSourceConf {conf: String, key: String},

    #[error("No {docs} directory for data source {key}")]
    NoDataSourceDocuments {docs: PathBuf, key: String},

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Provider(#[from] provider::DeserializeError),
    #[error(transparent)]
    Loader(#[from] loader::Error),
}

type Result<T> = std::result::Result<T, Error>;

static DATASOURCE_TOML: &str = "datasource.toml";
static DOCUMENTS: &str = "documents";

pub fn get_datasource_documents_path<P: AsRef<Path>>(source: P) -> PathBuf {
    let mut pth = source.as_ref().to_path_buf();
    pth.push(DOCUMENTS);
    pth
}

#[derive(Debug)]
pub struct DataSource {
    pub source: PathBuf,
    pub config: DataSourceConfig,
    pub all: BTreeMap<String, Value>,
    pub indices: BTreeMap<String, ValueIndex>,
}

#[derive(Eq, Debug, Serialize, Deserialize)]
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

#[derive(Debug)]
pub struct ValueIndex {
    pub documents: BTreeMap<IndexKey, Vec<String>>,
}

impl ValueIndex {
    pub fn to_keys(&self, key_type: &KeyType) -> Vec<Value> {
        return self
            .documents
            .keys()
            .map(|k| {
                match key_type {
                    KeyType::Full => {
                        to_value(k).unwrap()
                    },
                    KeyType::Name => {
                        to_value(&k.name).unwrap()
                    },
                    KeyType::Value => {
                        to_value(&k.value).unwrap()
                    }
                }
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

    fn map_entry(
        &self,
        k: &IndexKey,
        v: &Vec<String>,
        include_docs: bool,
        docs: &BTreeMap<String, Value>,
        query: &IndexQuery) -> Value {

        let id = slug::slugify(&k.name);
        let mut m = Map::new();

        m.insert("id".to_string(), json!(&id));
        m.insert("key".to_string(), json!(&k));

        if include_docs {
            if query.is_unique() && v.len() == 1 {
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
    }

    pub fn from_query(&self, query: &IndexQuery, docs: &BTreeMap<String, Value>) -> Vec<Value> {
        let include_docs = query.include_docs.is_some() && query.include_docs.unwrap();
        let desc = query.desc.is_some() && query.desc.unwrap();
        let offset = if let Some(ref offset) = query.offset { offset.clone() } else { 0 };
        let limit = if let Some(ref limit) = query.limit { limit.clone() } else { 0 };

        let iter: Box<dyn Iterator<Item = (usize, (&IndexKey, &Vec<String>))>> = if desc {
            // Note the enumerate() must be after rev() for the limit logic
            // to work as expected when DESC is set
            Box::new(self.documents.iter()
                .rev()
                .enumerate()
                .skip(offset))
        } else {
            Box::new(self.documents.iter()
                .enumerate()
                .skip(offset))
        };

        let mut items: Vec<Value> = Vec::new();
        for (i, (k, v)) in iter {
            if limit > 0 && i >= limit {
                break; 
            }

            let val = self.map_entry(k, v, include_docs, docs, query);
            items.push(val);
        }

        items
    }
}

#[derive(Debug, Default)]
pub struct DataSourceMap {
    pub map: BTreeMap<String, DataSource>,
}

impl DataSourceMap {
    pub fn get_datasource_config_path<P: AsRef<Path>>(source: P) -> PathBuf {
        let mut pth = source.as_ref().to_path_buf();
        pth.push(DATASOURCE_TOML);
        pth
    }

    fn to_data_source(path: &PathBuf, config: DataSourceConfig) -> DataSource {
        let all: BTreeMap<String, Value> = BTreeMap::new();
        let indices: BTreeMap<String, ValueIndex> = BTreeMap::new();
        DataSource {
            source: path.to_path_buf(),
            all,
            indices,
            config,
        }
    }

    // Expand out queries which generate multiple new pages, these can 
    // be each iterators or pagination queries
    pub fn expand(
        config: &Config,
        options: &RuntimeOptions,
        info: &mut CollateInfo,
        map: &DataSourceMap) -> Result<()> {

        for (q, p) in info.queries.iter() {
            let each = q.to_each_vec();

            if each.is_empty() {
                continue;
            }

            // Should have raw page data - note that we remove
            // the page as it is being used as an iterator
            let page = info.pages.remove(p).unwrap();

            let mut rewrite_index = options.settings.should_rewrite_index();
            // Override with rewrite-index page level setting
            if let Some(val) = page.rewrite_index {
                rewrite_index = val;
            }

            for each_query in each.iter() {
                let idx = map.query_index(each_query)?;

                //println!("Process each query for file {:?}", p.display());

                for doc in &idx {
                    let mut item_data = page.clone();
                    //println!("Got document {:?}", doc);

                    if let Some(id) = doc.get("id") {
                        if let Some(id) = id.as_str() {
                            if doc.is_object() {
                                let map = doc.as_object().unwrap();
                                for (k, v) in map {
                                    item_data.extra.insert(k.clone(), json!(v));
                                }
                            } else {
                                return Err(Error::DataSourceDocumentNotAnObject);
                            }

                            // Mock a source file to build a destination
                            // respecting the clean URL setting
                            let mut mock = p.parent().unwrap().to_path_buf();
                            mock.push(&id);
                            if let Some(ext) = p.extension() {
                                mock.set_extension(ext);
                            }

                            let mut file_info = FileInfo::new(
                                config,
                                options,
                                &mock,
                                true,
                            );

                            let file_opts = FileOptions {
                                rewrite_index,
                                base_href: &options.settings.base_href,
                                ..Default::default()
                            };

                            let dest = file_info.destination(&file_opts)?;

                            //info!("{} -> {}", &id, dest.display());
                            //info!("{} -> {}", mock.display(), &dest.display());

                            item_data.seal(
                                &dest,
                                config,
                                options,
                                &file_info)?;

                            // Assign the template so it is preferred over the mock
                            // synthetic path used to generate a destination
                            let file_ctx = item_data.file.as_mut().unwrap();
                            file_ctx.source = p.to_path_buf();

                            println!("Adding for mock path {:?}", mock.display());
                            println!("Using template path {:?}", file_ctx.source.display());

                            info.pages.entry(Arc::new(mock)).or_insert(item_data);
                        }
                    } else {
                        return Err(Error::DataSourceDocumentNoId);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn load(
        config: &Config,
        options: &RuntimeOptions,
        collation: &mut CollateInfo) -> Result<DataSourceMap> {

        let mut map: BTreeMap<String, DataSource> = BTreeMap::new();

        // Load data source configurations
        DataSourceMap::load_configurations(&mut map, options)?;

        // Map configurations for collations
        if options.settings.should_collate() {
            if let Some(ref sources) = config.collate {
                for (k, v) in sources {
                    let from = if v.from.is_some() {
                        v.from.as_ref().unwrap().clone()
                    } else {
                        options.source.clone() 
                    };

                    let mut cfg = v.clone();
                    if cfg.kind.is_none() {
                        cfg.kind = Some(Default::default());
                    }
                    if cfg.provider.is_none() {
                        cfg.provider = Some(Default::default());
                    }

                    let data_source = DataSourceMap::to_data_source(&from, cfg);
                    map.insert(k.to_string(), data_source);
                }
            }
        }

        // Load the documents for each configuration
        DataSourceMap::load_documents(&mut map, config, options, collation).await?;

        // Configure defaults
        DataSourceMap::configure_defaults(&mut map)?;

        // Create the indices
        DataSourceMap::load_index(&mut map)?;

        Ok(DataSourceMap { map })
    }

    fn load_configurations(map: &mut BTreeMap<String, DataSource>, options: &RuntimeOptions) -> Result<()> {
        let src = options.get_data_sources_path();
        if src.exists() && src.is_dir() {
            let contents = src.read_dir()?;
            DataSourceMap::load_config(map, contents)?;
        }
        Ok(())
    }

    fn load_config(map: &mut BTreeMap<String, DataSource>, dir: ReadDir) -> Result<()> {
        for f in dir {
            let mut path = f?.path();
            if path.is_dir() {
                if let Some(nm) = path.file_name() {
                    let key = nm.to_string_lossy().into_owned();
                    let conf = DataSourceMap::get_datasource_config_path(&path);
                    if !conf.exists() || !conf.is_file() {
                        return Err(Error::NoDataSourceConf {
                            conf: DATASOURCE_TOML.to_string(),
                            key
                        });
                    }

                    let contents = utils::fs::read_string(conf)?;
                    let config: DataSourceConfig = toml::from_str(&contents)?;

                    // For document providers there must be a documents directory
                    if let Some(SourceProvider::Documents) = config.provider {
                        // Respect from when set
                        if let Some(ref from) = config.from {
                            path.push(from); 
                        // Otherwise use the documents convention
                        } else {
                            let documents = get_datasource_documents_path(&path);
                            path = documents;
                        }
                    }

                    let data_source = DataSourceMap::to_data_source(&path.to_path_buf(), config);
                    map.insert(key, data_source);
                }
            }
        }

        Ok(())
    }

    async fn load_documents(
        map: &mut BTreeMap<String, DataSource>,
        config: &Config,
        options: &RuntimeOptions,
        collation: &CollateInfo) -> Result<()> {

        for (k, g) in map.iter_mut() {

            if !g.source.exists() || !g.source.is_dir() {
                return Err(Error::NoDataSourceDocuments {
                    docs: g.source.clone(),
                    key: k.to_string(),
                });
            }

            info!("{} < {}", k, g.source.display());

            let req = provider::LoadRequest {
                strategy: identifier::Strategy::FileName,
                kind: g.config.kind.as_ref().unwrap().clone(),
                provider: g.config.provider.as_ref().unwrap().clone(),
                source: &g.source,
                config,
                options,
                collation,
            };

            g.all = provider::Provider::load(req).await?;
        }
        Ok(())
    }

    // Configure the default keys
    fn configure_defaults(map: &mut BTreeMap<String, DataSource>) -> Result<()> {
        for (_, generator) in map.iter_mut() {
            if generator.config.index.is_none() {
                generator.config.index = Some(HashMap::new());
            }

            if let Some(ref mut index) = generator.config.index.as_mut() {
                // Inherit key from index name
                for (k, v) in index.iter_mut() {
                    if v.key.is_none() {
                        v.key = Some(k.clone());
                    }
                }
            }
        }
        Ok(())
    }

    fn find_field<S: AsRef<str>>(field: S, parent: &Value) -> Value {
        match parent {
            Value::Object(ref map) => {
                if let Some(val) = map.get(field.as_ref()) {
                    return val.clone();
                }
            },
            Value::Array(ref list) => {
                if let Ok(index) = field.as_ref().parse::<usize>() {
                    if !list.is_empty() && index < list.len() {
                        return list[index].clone();
                    }
                }
            },
            _ => {}
        } 
        Value::Null
    }

    fn find_value_for_key<S: AsRef<str>>(needle: S, doc: &Value) -> Value {
        #[allow(unused_assignments)]
        let mut parent = Value::Null;

        let parts = needle.as_ref()
            .split(".")
            .map(|p| p.to_string())
            .enumerate()
            .collect::<Vec<_>>();

        match doc {
            Value::Object(ref _map) => {
                let mut current: &Value = doc;
                for (i, part) in parts.iter() {
                    if *i == parts.len() - 1 {
                        return DataSourceMap::find_field(&part, current)
                    } else {
                        parent = DataSourceMap::find_field(&part, current);
                        if let Value::Null = parent {
                            break;
                        }
                        current = &parent;
                    }
                }
            },
            _ => {}
        }
        Value::Null
    }

    fn get_sort_key_for_value<S: AsRef<str>>(id: S, key_val: &Value) -> String {
        match key_val {
            Value::String(ref s) => {
                return s.to_string() 
            },
            _ => {}

        }
        id.as_ref().to_string()
    }

    fn load_index(map: &mut BTreeMap<String, DataSource>) -> Result<()> {
        let type_err = Err(Error::IndexKeyType);

        for (_, generator) in map.iter_mut() {
            let index = generator.config.index.as_ref().unwrap();

            for (name, def) in index {
                let identity = def.identity.is_some() && def.identity.unwrap();
                let key = def.key.as_ref().unwrap();
                let group = def.group.is_some() && def.group.unwrap();

                let mut values = ValueIndex { documents: BTreeMap::new() };

                for (id, document) in &generator.all {
                    let key_val = if identity {
                        Value::String(id.to_string())
                    } else {
                        DataSourceMap::find_value_for_key(key, document)
                    };

                    if let Value::Null = key_val {
                        continue;
                    }

                    let default_key = IndexKey {
                        name: DataSourceMap::get_sort_key_for_value(id, &key_val),
                        value: key_val.clone(),
                    };

                    if !group {
                        let items = values.documents
                            .entry(default_key)
                            .or_insert(Vec::new());
                        items.push(id.to_string());

                    } else {
                        let mut candidates: Vec<&str> = Vec::new();

                        if !key_val.is_string() && !key_val.is_array() {
                            return type_err;
                        }

                        if let Some(s) = key_val.as_str() {
                            candidates.push(s);
                        }else if let Some(arr) = key_val.as_array() {
                            for val in arr {
                                if let Some(s) = val.as_str() {
                                    candidates.push(s);
                                } else {
                                    return type_err;
                                }
                            }
                        }

                        for s in candidates {
                            let index_key = IndexKey {
                                name: s.to_string(),
                                value: key_val.clone(),
                            };

                            //println!("Creating index entry with key {:?}", s);
                            let items = values.documents
                                .entry(index_key)
                                .or_insert(Vec::new());

                            items.push(id.clone());
                        }
                    }
                }
                generator.indices.insert(name.clone(), values);
            }

            //for (k, idx) in &generator.indices {
                //println!("Index {:#?} for {:?}", idx, k);
            //}
        }
        Ok(())
    }

    pub fn query_index(&self, query: &IndexQuery) -> Result<Vec<Value>> {
        let name = &query.name;
        let idx_name = &query.index;
        let values = query.values.is_some() && query.values.unwrap();

        if let Some(generator) = self.map.get(name) {
            if let Some(idx) = generator.indices.get(idx_name) {
                if query.keys.is_some() {
                    return Ok(idx.to_keys(query.keys.as_ref().unwrap()));
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
}
