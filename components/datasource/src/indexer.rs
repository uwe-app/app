use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::fs::ReadDir;
use std::path::Path;
use std::path::PathBuf;

use log::info;
use serde_json::{Map, Value};

use collator::CollateInfo;
use config::{Config, RuntimeOptions};
use config::indexer::{
    IndexQuery,
    IndexKey,
    KeyType,
    KeyResult,
    SourceProvider,
    DataSource as DataSourceConfig,
    QueryValue,
    QueryResult};

use crate::{Error, Result};
use crate::identifier;
use crate::provider;

pub type QueryCache = HashMap<IndexQuery, Vec<QueryResult>>;

static DATASOURCE_TOML: &str = "datasource.toml";
static DOCUMENTS: &str = "documents";

static IDENTITY: &str = "id";
static NAME: &str = "name";
static PATH: &str = "path";
//static KEY: &str = "key";

pub fn get_datasource_documents_path<P: AsRef<Path>>(source: P) -> PathBuf {
    let mut pth = source.as_ref().to_path_buf();
    pth.push(DOCUMENTS);
    pth
}

#[derive(Debug)]
pub struct DataSource {
    pub source: PathBuf,
    pub config: DataSourceConfig,
    pub all: BTreeMap<String, Arc<Value>>,
    pub indices: BTreeMap<String, ValueIndex>,
}

#[derive(Debug)]
pub struct ValueIndex {
    pub documents: Vec<(IndexKey, String, Arc<Value>)>,
}

impl ValueIndex {

    fn get_key_result(&self, key: &IndexKey, key_type: &KeyType) -> KeyResult {
        match key_type {
            KeyType::Full => {
                KeyResult::Full(key.clone())
            }
            KeyType::Name => {
                KeyResult::Name(key.name.clone())
            }
            KeyType::Value => {
                KeyResult::Value(key.value.clone())
            }
        } 
    }

    fn get_identity(&self, slug: &str, id: &str, _key: &KeyResult) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(PATH.to_string(), Value::String(slug.to_string()));
        m.insert(NAME.to_string(), Value::String(id.to_string()));
        //m.insert(KEY.to_string(), to_value(key.clone()).unwrap());
        m
    }

    fn with_identity(&self, doc: &mut Value, slug: &str, id: &str, key: &KeyResult) {
        if doc.is_object() {
            let obj = doc.as_object_mut().unwrap();
            let ident = self.get_identity(slug, id, key);
            obj.insert(IDENTITY.to_string(), Value::Object(ident));
        }
    }

    fn map_entry(
        &self,
        k: &IndexKey,
        mut v: &mut Value,
        id: &str,
        query: &IndexQuery,
        include_docs: bool,
        ) -> QueryResult {

        let slug = slug::slugify(&k.name);

        //let unique = query.unique.is_some() && query.unique.unwrap();
        let keys = query.keys.is_some() && query.keys.unwrap();
        let key_type = query.key_type.as_ref().unwrap();
        let key = self.get_key_result(k, &key_type);

        if keys {
            return QueryResult { id: None, key: Some(key), value: None };
        }

        let value = if include_docs {
            //if unique && v.len() == 1
            //{
                //let mut doc = &v[0];
                //let mut doc = docs.get(id).unwrap();
                self.with_identity(&mut v, &slug, id, &key);
                Some(QueryValue::One(v.clone())) 
            //} else {
                //let docs = v
                    //.iter()
                    //.filter(|s| docs.contains_key(&**s))
                    //.map(|id| {
                        //let mut doc = docs.get(id).unwrap();
                        //self.with_identity(&mut doc, &slug, id, &key);
                        //doc
                    //})
                    //.collect::<Vec<_>>();

                //Some(QueryValue::Many(docs))
            //}

        } else {
            None 
        };

        let res = QueryResult {
            id: Some(slug),
            key: Some(key),
            value,
        };

        res
    }

    pub fn from_query(&self, query: &IndexQuery) -> Vec<QueryResult> {
        let include_docs = query.include_docs.is_some() && query.include_docs.unwrap();
        let desc = query.desc.is_some() && query.desc.unwrap();
        let offset = if let Some(ref offset) = query.offset { offset.clone() } else { 0 };
        let limit = if let Some(ref limit) = query.limit { limit.clone() } else { 0 };

        let mut index_docs = self.documents.clone();

        // Sorting needs to happen before enumeration so currently involves
        // a copy of the keys and injection of the `sort` field used to order
        // the index, obviously this can be done much better.
        if let Some(ref sort_key) = query.sort {

            //println!("Sort start {}", index_docs.len());

            index_docs.sort_by(|a, b| {
                let (_ak, _aid, arc) = a;
                let (_bk, _bid, brc) = b;

                let doc_a = &*arc.clone();
                let doc_b = &*brc.clone();

                let sort_a = config::path::find_path(sort_key, doc_a); 
                let sort_b = config::path::find_path(sort_key, doc_b); 

                let str_a = sort_a.to_string();
                let str_b = sort_b.to_string();

                str_a.partial_cmp(&str_b).unwrap()
            });
        }

        let iter: Box<dyn Iterator<Item = (usize, &(IndexKey, String, Arc<Value>))>> = if desc {
            // Note the enumerate() must be after rev() for the limit logic
            // to work as expected when DESC is set
            Box::new(index_docs.iter()
                .rev()
                .enumerate()
                .skip(offset))
        } else {
            Box::new(index_docs.iter()
                .enumerate()
                .skip(offset))
        };

        let mut items: Vec<QueryResult> = Vec::new();
        for (i, (k, id, v)) in iter {
            if limit > 0 && i >= limit {
                break; 
            }

            let doc = &*v.clone();
            let mut new_doc = doc.clone();
            let val = self.map_entry(k, &mut new_doc, id, query, include_docs);
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
        let all: BTreeMap<String, Arc<Value>> = BTreeMap::new();
        let indices: BTreeMap<String, ValueIndex> = BTreeMap::new();
        DataSource {
            source: path.to_path_buf(),
            all,
            indices,
            config,
        }
    }

    pub fn get_cache() -> QueryCache {
        HashMap::new()
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

                let mut values = ValueIndex { documents: Vec::new() };

                for (id, document) in &generator.all {
                    let key_val = if identity {
                        Value::String(id.to_string())
                    } else {
                        config::path::find_path(key, document)
                    };

                    if let Value::Null = key_val {
                        continue;
                    }

                    let default_key = IndexKey {
                        name: DataSourceMap::get_sort_key_for_value(id, &key_val),
                        value: key_val.clone(),
                    };

                    if !group {
                        values.documents.push((default_key, id.clone(), Arc::clone(document)));

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

                        //for s in candidates {
                            //let index_key = IndexKey {
                                //name: s.to_string(),
                                //value: key_val.clone(),
                            //};

                            //let items = values.documents
                                //.entry(index_key)
                                //.or_insert(Vec::new());

                            //items.push(id.clone());
                            //values.documents.push((index_key, vec![]));
                        //}
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


    fn get_result_set(
        &self,
        _ds: &DataSource,
        idx: &ValueIndex,
        query: &IndexQuery,
        cache: &mut QueryCache) -> Result<Vec<QueryResult>> {

        if let Some(ref cached) = cache.get(query) {
            return Ok(cached.to_vec())
        }

        let res = idx.from_query(query);
        cache.entry(query.clone()).or_insert(res.clone());

        Ok(res)
    }

    pub fn query_index(&self, query: &IndexQuery, cache: &mut QueryCache) -> Result<Vec<QueryResult>> {
        let name = &query.name;
        let idx_name = &query.index;

        if let Some(generator) = self.map.get(name) {
            if let Some(idx) = generator.indices.get(idx_name) {
                self.get_result_set(generator, idx, query, cache)
            } else {
                return Err(Error::NoIndex(idx_name.to_string()));
            }
        } else {
            return Err(Error::NoDataSource(name.to_string()));
        }
    }
}
