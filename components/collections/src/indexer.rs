use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use log::info;
use serde_json::{Map, Value};

use collator::CollateInfo;
use config::indexer::{
    DataProvider, IndexKey, IndexQuery, KeyResult, KeyType, QueryResult,
    QueryValue,
};
use config::{Config, RuntimeOptions};
use utils::json_path;

use crate::{identifier, provider, Error, Result};

pub type QueryCache = HashMap<IndexQuery, Vec<QueryResult>>;
pub type IndexValue = (IndexKey, Arc<Value>);
pub type Index = Vec<IndexValue>;

const IDENTITY: &str = "id";
const NAME: &str = "name";
const PATH: &str = "path";
const DOC: &str = "doc";
const IDENTITY_KEY: &str = "*";

#[derive(Debug)]
pub struct CollectionDataBase {
    source: PathBuf,
    config: DataProvider,
    all: BTreeMap<String, Arc<Value>>,
    indices: BTreeMap<String, ValueIndex>,
}

impl CollectionDataBase {
    pub fn new(source: PathBuf, config: DataProvider) -> Self {
        CollectionDataBase {
            source,
            all: BTreeMap::new(),
            indices: BTreeMap::new(),
            config,
        }
    }

    pub fn source(&self) -> &PathBuf {
        &self.source
    }

    pub fn data_provider(&self) -> &DataProvider {
        &self.config
    }

    /// Build a single database; loading documents from disc
    /// and computing indices.
    pub async fn build(
        &mut self,
        db_name: &str,
        config: &Config,
        options: &RuntimeOptions,
        collation: &CollateInfo,
    ) -> Result<()> {
        // Ensure the database is pristine
        self.clear();

        // Load the documents for the database
        self.load_provider(db_name, config, options, collation)
            .await?;

        // Compute the indices for the new database
        self.load_indices(db_name)?;

        Ok(())
    }

    async fn load_provider(
        &mut self,
        db_name: &str,
        config: &Config,
        options: &RuntimeOptions,
        collation: &CollateInfo,
    ) -> Result<()> {
        if !self.source.exists() {
            return Err(Error::NoCollectionDocuments {
                docs: self.source.clone(),
                key: db_name.to_string(),
            });
        }

        info!("Load {} < {}", db_name, self.source.display());

        let req = provider::LoadRequest {
            strategy: identifier::Strategy::FileName,
            kind: self.config.kind(),
            provider: self.config.source_provider(),
            source: &self.source,
            definition: &self.config,
            config,
            options,
            collation,
        };

        // Load all the documents into the db
        self.all = provider::Provider::load(req).await?;

        Ok(())
    }

    fn load_indices(&mut self, db_name: &str) -> Result<()> {
        let index = self.config.index.as_ref().unwrap();

        for (name, def) in index {
            info!("Build index {} / {}", db_name, name);

            let key = def.key.clone();
            let identity = key == IDENTITY_KEY;

            let mut values = ValueIndex {
                documents: Vec::new(),
            };

            for (id, document) in self.all.iter() {
                let key_val = if identity {
                    Value::String(id.to_string())
                } else {
                    json_path::find_path(&key, document)
                };

                if let Value::Null = key_val {
                    continue;
                }

                let default_key = IndexKey {
                    id: id.clone(),
                    name: id.clone(),
                    doc_id: id.clone(),
                    sort: CollectionsMap::get_sort_key_for_value(id, &key_val),
                    value: key_val.clone(),
                };

                values.documents.push((default_key, Arc::clone(document)));
            }

            // Sort using default key
            values.documents.sort_by(|a, b| {
                let (ak, _arc) = a;
                let (bk, _brc) = b;
                ak.partial_cmp(&bk).unwrap()
            });

            self.indices.insert(name.clone(), values);
        }

        /*
        for (k, idx) in &self.indices {
            if k == "all" {
                println!("Index {:#?} for {:?}", idx, k);
            }
        }
        */

        Ok(())
    }

    pub fn clear(&mut self) {
        self.all.clear();
        self.indices.clear();
    }
}

#[derive(Debug)]
pub struct ValueIndex {
    pub documents: Index,
}

impl ValueIndex {
    fn get_key_result(&self, key: &IndexKey, key_type: &KeyType) -> KeyResult {
        match key_type {
            KeyType::Full => KeyResult::Full(key.clone()),
            KeyType::Id => KeyResult::Name(key.id.clone()),
            KeyType::Value => KeyResult::Value(key.value.clone()),
        }
    }

    fn get_identity(&self, slug: &str, key: &IndexKey) -> Map<String, Value> {
        let mut m = Map::new();
        //m.insert(DOC.to_string(), Value::String(key.doc_id.to_string()));
        m.insert(DOC.to_string(), Value::String(key.doc_id.to_string()));
        m.insert(NAME.to_string(), Value::String(key.id.to_string()));
        m.insert(PATH.to_string(), Value::String(slug.to_string()));
        m
    }

    fn with_identity(&self, doc: &mut Value, slug: &str, key: &IndexKey) {
        let ident = self.get_identity(slug, key);

        if doc.is_object() {
            let obj = doc.as_object_mut().unwrap();
            obj.insert(IDENTITY.to_string(), Value::Object(ident));
        } else if doc.is_array() {
            let list = doc.as_array_mut().unwrap();
            for doc in list.iter_mut() {
                match doc {
                    Value::Object(_) => {
                        let obj = doc.as_object_mut().unwrap();
                        obj.insert(
                            IDENTITY.to_string(),
                            Value::Object(ident.clone()),
                        );
                    }
                    _ => {
                        // TODO: how to handle other types of data?
                    }
                }
            }
        }

        //Ok(())
    }

    fn map_entry(
        &self,
        k: &IndexKey,
        mut v: &mut Value,
        query: &IndexQuery,
        include_docs: bool,
    ) -> QueryResult {
        let slug = slug::slugify(&k.id);
        let keys = query.keys.is_some() && query.keys.unwrap();
        let key_type = query.key_type.as_ref().unwrap();
        let key = self.get_key_result(k, &key_type);

        if keys {
            return QueryResult {
                id: None,
                key: Some(key),
                value: None,
            };
        }

        let value = if include_docs {
            if query.group.is_none() {
                self.with_identity(&mut v, &slug, k);
            }
            Some(QueryValue::One(v.clone()))
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

    fn group_by(&self, input: &Index, query: &IndexQuery) -> Index {
        let mut idx = Vec::new();

        let group = query.group.as_ref().unwrap();
        let expand = group.expand.is_some() && group.expand.unwrap();

        let mut tmp: BTreeMap<String, (IndexKey, Value)> = BTreeMap::new();

        for v in input {
            let (key, arc) = v;

            let doc = &*arc.clone();
            let group_key = json_path::find_path(&group.path, doc);

            let candidates = if group_key.is_array() && expand {
                group_key.as_array().unwrap().to_vec()
            } else {
                vec![group_key]
            };

            for val in candidates {
                let val_key = if val.is_string() {
                    val.as_str().unwrap().to_string()
                } else {
                    val.to_string()
                };

                let mut new_key = key.clone();
                new_key.id = slug::slugify(&val_key);
                new_key.name = val_key.clone();

                let mut group_doc = doc.clone();
                self.with_identity(&mut group_doc, &new_key.id, &new_key);

                tmp.entry(val_key.clone())
                    .or_insert((new_key, Value::Array(vec![])));
                let (_, items) = tmp.get_mut(&val_key).unwrap();
                let list = items.as_array_mut().unwrap();
                list.push(group_doc);
            }
        }

        for (_, (key, value)) in tmp {
            idx.push((key, Arc::new(value)));
        }

        idx
    }

    pub fn from_query(&self, query: &IndexQuery) -> Vec<QueryResult> {
        let include_docs =
            query.include_docs.is_some() && query.include_docs.unwrap();
        let desc = query.desc.is_some() && query.desc.unwrap();
        let offset = if let Some(ref offset) = query.offset {
            offset.clone()
        } else {
            0
        };
        let limit = if let Some(ref limit) = query.limit {
            limit.clone()
        } else {
            0
        };

        let mut index_docs = self.documents.clone();

        if query.group.is_some() {
            index_docs = self.group_by(&index_docs, query);
        }

        // Sorting needs to happen before enumeration so currently involves
        // a copy of the keys and injection of the `sort` field used to order
        // the index, obviously this can be done much better.
        if let Some(ref sort_key) = query.sort {
            index_docs.sort_by(|a, b| {
                let (_ak, arc) = a;
                let (_bk, brc) = b;

                let doc_a = &*arc.clone();
                let doc_b = &*brc.clone();

                let sort_a = json_path::find_path(sort_key, doc_a);
                let sort_b = json_path::find_path(sort_key, doc_b);

                let str_a = sort_a.to_string();
                let str_b = sort_b.to_string();

                str_a.partial_cmp(&str_b).unwrap()
            });
        }

        let iter: Box<
            dyn Iterator<Item = (usize, &mut (IndexKey, Arc<Value>))>,
        > = if desc {
            // Note the enumerate() must be after rev() for the limit logic
            // to work as expected when DESC is set
            Box::new(index_docs.iter_mut().rev().enumerate().skip(offset))
        } else {
            Box::new(index_docs.iter_mut().enumerate().skip(offset))
        };

        let mut items: Vec<QueryResult> = Vec::new();
        for (i, (k, v)) in iter {
            if limit > 0 && i >= limit {
                break;
            }

            let doc = &*v.clone();
            let mut new_doc = doc.clone();
            let val = self.map_entry(k, &mut new_doc, query, include_docs);
            items.push(val);
        }

        items
    }
}

#[derive(Debug, Default)]
pub struct CollectionsMap {
    map: BTreeMap<String, CollectionDataBase>,
}

impl CollectionsMap {
    pub fn iter(
        &self,
    ) -> std::collections::btree_map::Iter<'_, String, CollectionDataBase> {
        self.map.iter()
    }

    pub fn map(&self) -> &BTreeMap<String, CollectionDataBase> {
        &self.map
    }

    pub fn map_mut(&mut self) -> &mut BTreeMap<String, CollectionDataBase> {
        &mut self.map
    }
}

impl CollectionsMap {
    /// Load database document providers and compute indices.
    pub async fn load(
        &mut self,
        config: &Config,
        options: &RuntimeOptions,
        collation: &mut CollateInfo,
    ) -> Result<()> {
        if let Some(ref db) = config.db {
            if let Some(ref sources) = db.load {
                for (db_name, provider) in sources {
                    let from = if let Some(ref from) = provider.from() {
                        options.source.join(from)
                    } else {
                        options.source.clone()
                    };

                    let mut db = CollectionDataBase::new(
                        from.to_path_buf(),
                        provider.clone(),
                    );

                    // Load the documents for the database and compute indices
                    db.build(db_name, config, options, collation).await?;

                    // Store for querying and live reload invalidation
                    self.map.insert(db_name.to_string(), db);
                }
            }
        }

        Ok(())
    }

    fn get_sort_key_for_value<S: AsRef<str>>(id: S, key_val: &Value) -> String {
        match key_val {
            Value::String(ref s) => return s.to_string(),
            _ => {}
        }
        id.as_ref().to_string()
    }

    fn get_result_set(
        &self,
        _ds: &CollectionDataBase,
        idx: &ValueIndex,
        query: &IndexQuery,
        cache: &mut QueryCache,
    ) -> Result<Vec<QueryResult>> {
        if let Some(ref cached) = cache.get(query) {
            return Ok(cached.to_vec());
        }

        let res = idx.from_query(query);

        //println!("Get result set from query {:#?}", query);
        //println!("Got result set {:#?}", res);

        cache.entry(query.clone()).or_insert(res.clone());

        Ok(res)
    }

    pub fn query_index(
        &self,
        query: &IndexQuery,
        cache: &mut QueryCache,
    ) -> Result<Vec<QueryResult>> {
        let name = &query.name;
        let idx_name = &query.index;

        if let Some(generator) = self.map.get(name) {
            if let Some(idx) = generator.indices.get(idx_name) {
                self.get_result_set(generator, idx, query, cache)
            } else {
                return Err(Error::NoIndex(idx_name.to_string()));
            }
        } else {
            return Err(Error::NoCollection(name.to_string()));
        }
    }
}
