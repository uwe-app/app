use std::path::PathBuf;
use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use serde::{Deserialize, Serialize};
use log::{info, warn};
use slug;

use crate::{
    utils,
    Error,
    JSON,
    DOCUMENTS,
    GENERATOR_TOML,
};

use crate::config::Config;

static ALL_INDEX: &str = "all";
static DEFAULT_PARAMETER: &str = "documents";
static DEFAULT_VALUE_PARAMETER: &str = "value";

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorConfig {
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
        return self.index == ALL_INDEX.to_string()
            || self.flat.is_some() && self.flat.unwrap();
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
pub struct Generator {
    pub site: PathBuf,
    pub source: PathBuf,
    pub config: GeneratorConfig,
    pub all: BTreeMap<String, Value>,
    pub indices: BTreeMap<String, ValueIndex>,
}

#[derive(Debug)]
pub struct ValueIndex {
    pub documents: BTreeMap<String, Vec<String>>,
}

impl ValueIndex {

    pub fn to_keys(&self) -> Vec<Value> {
        return self.documents
            .keys()
            .map(|k| {
                return json!(&k);
            })
            .collect::<Vec<_>>();
    }

    pub fn to_values(&self) -> Vec<Value> {
        return self.documents
            .values()
            .map(|v| {
                return json!(&v);
            })
            .collect::<Vec<_>>();
    }

    pub fn from_query(
        &self,
        query: &IndexQuery,
        docs: &BTreeMap<String, Value>) -> Vec<Value> {

        let include_docs = query.include_docs.is_some() && query.include_docs.unwrap();

        return self.documents
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

impl Generator {

    pub fn load(&mut self) -> Result<(), Error> {
        let mut data_dir = self.source.clone();
        data_dir.push(DOCUMENTS);
        match data_dir.read_dir() {
            Ok(contents) => {
                for e in contents {
                    match e {
                        Ok(entry) => {
                            let path = entry.path();

                            if let Some(ext) = path.extension() {
                                if ext == JSON {
                                    let contents = utils::read_string(&path)?;
                                    let document: Value =
                                        serde_json::from_str(&contents)?;
                                    if let Some(stem) = path.file_stem() {
                                        let name = stem.to_string_lossy().into_owned();
                                        let id = slug::slugify(&name);

                                        if self.all.contains_key(&id) {
                                            return Err(
                                                Error::new(
                                                    format!(
                                                        "Duplicate document id {} ({}.json)", &id, &name)));
                                        }

                                        self.all.insert(id, document);
                                    }
                                } 
                            }

                        },
                        Err(e) => return Err(Error::from(e))
                    }
                } 
            },
            Err(e) => return Err(Error::from(e))
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct GeneratorMap {
    pub map: BTreeMap<String, Generator>,
}

impl GeneratorMap {
    pub fn new() -> Self {
        let map: BTreeMap<String, Generator> = BTreeMap::new();
        GeneratorMap {
            map,
        } 
    }

    pub fn load(&mut self, source: PathBuf, config: &Config) -> Result<(), Error> {
        self.load_configurations(source, config)?;
        self.load_documents()?;
        self.configure_default_index()?;
        self.load_index()?;
        Ok(())
    }

    // Configure the default all index
    fn configure_default_index(&mut self) -> Result<(), Error> {
        for (_, generator) in self.map.iter_mut() {
            if generator.config.index.is_none() {
                generator.config.index = Some(BTreeMap::new());
            }

            let index = generator.config.index.as_ref().unwrap();

            // Complain on reserved index name
            if index.contains_key(ALL_INDEX) {
                return Err(
                    Error::new(
                        "The all index is reserved, choose another index name.".to_string()));
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
                    IndexRequest{key: Some(ALL_INDEX.to_string())});
            }
        }
        Ok(())
    }

    fn load_index(&mut self) -> Result<(), Error> {
        let type_err = Err(
            Error::new(format!("Type error building index, keys must be string values")));

        for (_, generator) in self.map.iter_mut() {
            let index = generator.config.index.as_ref().unwrap();

            for (name, def) in index {
                let key = def.key.as_ref().unwrap();
                let mut values = ValueIndex{documents: BTreeMap::new()};

                for (id, document) in &generator.all {
                    if name == ALL_INDEX {
                        let items = values.documents.entry(id.clone()).or_insert(Vec::new());
                        items.push(id.clone());
                        continue; 
                    }

                    if let Some(val) = document.get(&key) {
                        let mut candidates: Vec<&str> = Vec::new();

                        if !val.is_string() && !val.is_array() {
                            return type_err
                        }

                        if let Some(s) = val.as_str() {
                            candidates.push(s);
                        }

                        if let Some(arr) = val.as_array() {
                            for val in arr {
                                if let Some(s) = val.as_str() {
                                    candidates.push(s);
                                } else {
                                    return type_err
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
        //std::process::exit(1);
        Ok(())
    }

    pub fn query_index(&self, query: &IndexQuery) -> Result<Vec<Value>, Error> {
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
                return Err(Error::new(format!("Missing generator index '{}'", idx_name))) 
            }
        } else {
            return Err(Error::new(format!("Missing generator with name '{}'", name))) 
        }
    }

    fn load_documents(&mut self) -> Result<(), Error> {
        for (k, g) in self.map.iter_mut() {
            info!("{} < {}", k, g.source.display());
            g.load()?;
        }
        Ok(())
    }

    fn load_configurations(&mut self, source: PathBuf, config: &Config) -> Result<(), Error> {
        let src = config.get_generator_path(&source);

        if src.exists() && src.is_dir() {
            let result = src.read_dir();
            match result {
                Ok(contents) => {
                    for f in contents {
                        if let Ok(entry) = f {
                            let path = entry.path();
                            if path.is_dir() {
                                if let Some(nm) = path.file_name() {
                                    let key = nm.to_string_lossy().into_owned(); 
                                    let mut conf = path.to_path_buf().clone();
                                    conf.push(GENERATOR_TOML);
                                    if !conf.exists() || !conf.is_file() {
                                        return Err(
                                            Error::new(
                                                format!("No {} for generator {}", GENERATOR_TOML, key)));
                                    }

                                    let mut data = path.to_path_buf().clone();
                                    data.push(DOCUMENTS);
                                    if !data.exists() || !data.is_dir() {
                                        return Err(
                                            Error::new(
                                                format!("No {} directory for generator {}", DOCUMENTS, key)));
                                    }

                                    let contents = utils::read_string(conf)?;
                                    let config: GeneratorConfig = toml::from_str(&contents)?;

                                    let all: BTreeMap<String, Value> = BTreeMap::new();
                                    let indices: BTreeMap<String, ValueIndex> = BTreeMap::new();

                                    let generator = Generator {
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
                    }
                },
                Err(e) => return Err(Error::from(e))
            }
        }
        Ok(())
    }

}
