use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::sync::Mutex;

use slug;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use log::{info};

use crate::{
    utils,
    Error,
    GENERATOR,
    DOCUMENTS,
    GENERATOR_TOML,
};

lazy_static! {
    #[derive(Debug)]
    pub static ref GENERATOR_MAPPING: Mutex<BTreeMap<String, GeneratorUrlMapInfo>> = {
        Mutex::new(BTreeMap::new())
    };
}

#[derive(Debug)]
pub struct GeneratorUrlMapInfo {
    pub destination: String,
    pub ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorReference {
    pub name: String,
    pub index: String,
    pub parameter: Option<String>,
    pub include_docs: Option<bool>,
    pub keys: Option<bool>,
    pub each: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorIndexRequest {
    pub key: String,
    pub map: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorBuildConfig {
    // Destination output for generated pages relative to the site
    pub destination: String,
}

impl GeneratorBuildConfig {
    pub fn validate<P: AsRef<Path>>(&self, _dir: P) -> Result<(), Error> {
        let dest = Path::new(&self.destination);
        if dest.is_absolute() {
            return Err(
                Error::new(
                    format!("Generator destination '{}' must be relative path", self.destination)))
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorConfig {
    pub build: GeneratorBuildConfig,
    pub index: BTreeMap<String, GeneratorIndexRequest>,
}

impl GeneratorConfig {
    pub fn validate<P: AsRef<Path>>(&self, dir: P) -> Result<(), Error> {
        self.build.validate(dir)
    }
}

#[derive(Debug)]
pub struct Generator {
    pub site: PathBuf,
    pub source: PathBuf,
    pub config: GeneratorConfig,
    pub all: DocumentIndex,
    pub indices: BTreeMap<String, ValueIndex>,
}

#[derive(Debug, Serialize)]
pub struct DocumentIndex{
    pub documents: Vec<Box<SourceDocument>>,
}

impl DocumentIndex {
    pub fn to_value_vec(&self) -> Vec<Value> {
        return self.documents
            .iter()
            .map(|v| json!(v))
            .collect::<Vec<_>>();
    }
}

#[derive(Debug, Serialize, Default, Clone)]
pub struct SourceDocument {
    pub id: String,
    pub document: Value,
}

#[derive(Debug)]
pub struct ValueIndex {
    pub documents: Vec<IndexDocument>,
}

#[derive(Debug, Clone)]
pub struct IndexDocument {
    pub key: String,
    pub doc: Vec<Box<SourceDocument>>,
}

impl ValueIndex {
    pub fn to_value_vec(&self, keys: bool, include_docs: bool) -> Vec<Value> {
        return self.documents
            .iter()
            .map(|v| {
                if keys {
                    return json!(&v.key);
                }

                let mut m = Map::new();
                m.insert("id".to_string(), json!(slug::slugify(&v.key)));
                m.insert("key".to_string(), json!(&v.key));
                if include_docs {
                    let docs = v.doc
                        .iter()
                        .map(|s| {
                            let mut m = Map::new();
                            m.insert("id".to_string(), json!(&s.id));
                            m.insert("document".to_string(), json!(&s.document));
                            m
                        })
                        .collect::<Vec<_>>();

                    m.insert("value".to_string(), json!(&docs));
                }
                json!(&m)
            })
            .collect::<Vec<_>>();
    }
}

impl Generator {

    pub fn load(&mut self, ids: &mut Vec<String>) -> Result<(), Error> {

        let mut site_dir = self.site.clone();
        site_dir.push(&self.config.build.destination);

        let mut data_dir = self.source.clone();
        data_dir.push(DOCUMENTS);
        match data_dir.read_dir() {
            Ok(contents) => {
                for e in contents {
                    match e {
                        Ok(entry) => {
                            let path = entry.path();
                            let contents = utils::read_string(&path)?;
                            let document: Value =
                                serde_json::from_str(&contents)?;
                            if let Some(stem) = path.file_stem() {
                                let name = stem.to_string_lossy().into_owned();
                                let id = slug::slugify(name);
                                ids.push(id.clone());
                                self.all.documents.push(Box::new(SourceDocument{id, document}));
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

    pub fn load(&mut self, source: PathBuf) -> Result<(), Error> {
        self.load_configurations(source)?;
        self.load_documents()?;
        self.load_index()?;
        Ok(())
    }

    fn load_index(&mut self) -> Result<(), Error> {

        let type_err = Err(
            Error::new(format!("Type error building index, keys must be string values")));

        for (_, generator) in self.map.iter_mut() {
            let all = &generator.all;

            // Collect identifiers grouping first by index key
            // and then by the values for the referenced fields
            let mut caches: BTreeMap<String, BTreeMap<String, Vec<Box<SourceDocument>>>> = BTreeMap::new();

            for (name, def) in &generator.config.index {
                let values = ValueIndex{documents: Vec::new()};
                let key = &def.key;
                let cache = caches.entry(name.clone()).or_insert(BTreeMap::new());

                for doc in &all.documents {
                    let document = &doc.document;

                    if key == "@id" {
                        let items = cache.entry(doc.id.clone()).or_insert(Vec::new());
                        items.push(doc.clone());
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
                            let items = cache.entry(s.to_string()).or_insert(Vec::new());
                            items.push(doc.clone());
                        }
                    }
                }

                generator.indices.insert(name.clone(), values);
            }

            for (k, v) in caches {
                let idx = generator.indices.get_mut(&k).unwrap();
                for (key, val) in v {
                    let idx_doc = IndexDocument {
                        key: key,
                        doc: val,
                    };
                    idx.documents.push(idx_doc);
                }
            }

            //for (k, idx) in &generator.indices {
                //println!("index key {:?}", k);
                //println!("{}", serde_json::to_string_pretty(&idx.to_value_vec(false, true)).unwrap());
            //}
        }
        Ok(())
    }

    pub fn find_generator_index(&self, generator: &GeneratorReference) -> Result<Vec<Value>, Error> {
        let name = &generator.name;
        let idx_name = &generator.index;

        let include_docs = generator.include_docs.is_some() && generator.include_docs.unwrap();
        let keys = generator.keys.is_some() && generator.keys.unwrap();

        if let Some(generator) = self.map.get(name) {
            if idx_name == "all" {
                return Ok(generator.all.to_value_vec());
            } else {
                if let Some(idx) = generator.indices.get(idx_name) {
                    return Ok(idx.to_value_vec(keys, include_docs));
                } else {
                    return Err(Error::new(format!("Missing generator index '{}'", idx_name))) 
                }
            }
        } else {
            return Err(Error::new(format!("Missing generator with name '{}'", name))) 
        }
    }

    fn load_documents(&mut self) -> Result<(), Error> {
        let mut mapping = GENERATOR_MAPPING.lock().unwrap();
        for (k, g) in self.map.iter_mut() {
            let item = mapping.get_mut(k).unwrap();
            let mut ids = &mut item.ids;
            g.load(&mut ids)?;
            info!("{} < {}", k, g.source.display());
        }
        Ok(())
    }

    fn load_configurations(&mut self, source: PathBuf) -> Result<(), Error> {
        let mut mapping = GENERATOR_MAPPING.lock().unwrap();

        let mut src = source.clone();
        src.push(GENERATOR);

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

                                    if let Err(e) = config.validate(&path) {
                                        return Err(e) 
                                    }

                                    let all = DocumentIndex{documents: Vec::new()};
                                    let indices: BTreeMap<String, ValueIndex> = BTreeMap::new();

                                    let generator = Generator {
                                        site: source.clone(),
                                        source: path.to_path_buf(),
                                        all,
                                        indices,
                                        config,
                                    };

                                    let gmi = GeneratorUrlMapInfo {
                                        destination: generator.config.build.destination.clone(),
                                        ids: Vec::new(),
                                    };
                                    mapping.insert(key.clone(), gmi);

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

