use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use log::{info};

use crate::{
    utils,
    Error,
    BuildOptions,
    TEMPLATE,
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
    pub copy_json: bool,
    pub json_index: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorIndexRequest {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorBuildConfig {
    // Destination output for generated pages relative to the site
    pub destination: String,
    // Name of the template used for page generation
    pub template: String,
    pub index: Option<Vec<GeneratorIndexRequest>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorJsonConfig {
    // Copy documents to the build
    pub copy: bool,
    // Whether the JSON contents just contains
    // an index pointing to the copied files
    pub index_slim: bool,
    // Output file for the index
    pub index_file: Option<String>,
}

impl GeneratorBuildConfig {
    pub fn validate<P: AsRef<Path>>(&self, dir: P) -> Result<(), Error> {
        let f = dir.as_ref();

        let mut t = f.to_path_buf();
        t.push(&self.template);
        if !t.exists() || !t.is_file() {
            return Err(
                Error::new(
                    format!("Generator template '{}' is not a file", self.template)))
        }

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
    pub json: Option<GeneratorJsonConfig>,
}

impl GeneratorConfig {
    pub fn validate<P: AsRef<Path>>(&self, dir: P) -> Result<(), Error> {
        self.build.validate(dir)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Generator {
    pub site: PathBuf,
    pub source: PathBuf,
    pub config: GeneratorConfig,
    pub indices: BTreeMap<String, IndexType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IndexType{
    Documents(DocumentIndex),
    Values(ValueIndex),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentIndex{
    pub documents: Vec<SourceDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValueIndex{
    pub documents: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceDocument {
    pub id: String,
    pub document: Value,
}

impl Generator {

    pub fn load(&mut self, ids: &mut Vec<String>) -> Result<(), Error> {
        let all = self.indices.get_mut("all").unwrap();

        match all {
            IndexType::Documents(ref mut all) => {
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
                                        let id = stem.to_string_lossy().into_owned();
                                        ids.push(id.clone());
                                        all.documents.push(SourceDocument{id, document});
                                    }
                                },
                                Err(e) => return Err(Error::from(e))
                            }
                        } 
                    },
                    Err(e) => return Err(Error::from(e))
                }
            
            },
            _ => {}
        }

        Ok(())
    }
}

fn load_documents(generators: &mut BTreeMap<String, Generator>) -> Result<(), Error> {
    let mut mapping = GENERATOR_MAPPING.lock().unwrap();
    for (k, g) in generators.iter_mut() {
        let item = mapping.get_mut(k).unwrap();
        let mut ids = &mut item.ids;
        g.load(&mut ids)?;
        info!("{} < {}", k, g.source.display());
    }
    Ok(())
}

fn load_configurations(opts: &BuildOptions, generators: &mut BTreeMap<String, Generator>) -> Result<(), Error> {
    let mut mapping = GENERATOR_MAPPING.lock().unwrap();

    let mut src = opts.source.clone();
    src.push(TEMPLATE);
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

                                let mut copy_json = false;
                                let mut json_index = None;
                                if let Some(json) = &config.json {
                                    copy_json = json.copy; 
                                    json_index = json.index_file.clone();
                                }

                                let all = IndexType::Documents(DocumentIndex{documents: Vec::new()});
                                let mut indices: BTreeMap<String, IndexType> = BTreeMap::new();
                                indices.insert("all".to_string(), all);

                                let generator = Generator {
                                    site: opts.source.clone(),
                                    source: path.to_path_buf(),
                                    indices,
                                    config,
                                };

                                let gmi = GeneratorUrlMapInfo {
                                    destination: generator.config.build.destination.clone(),
                                    ids: Vec::new(),
                                    copy_json,
                                    json_index,
                                };
                                mapping.insert(key.clone(), gmi);

                                generators.insert(key, generator);

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

fn build_index(generators: &mut BTreeMap<String, Generator>) -> Result<(), Error> {
    for (k, generator) in &mut generators.iter_mut() {
        if let Some(index) = &generator.config.build.index {
            let mut indices = &mut generator.indices;
            let all = indices.get("all").unwrap();

            for def in index {
                println!("got index def {:?}", def);
                // Documents for this index
                //let mut docs: Vec<SourceDocument> = Vec::new();
            }

            //println!("build index for {:?}", k);
            //println!("build index for {:?}", indices);
        }
    }
    Ok(())
}

pub fn load(opts: &BuildOptions) -> Result<BTreeMap<String, Generator>, Error> {
    let mut map: BTreeMap<String, Generator> = BTreeMap::new();
    load_configurations(opts, &mut map)?;
    load_documents(&mut map)?;
    build_index(&mut map)?;
    Ok(map)
}
