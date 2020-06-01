use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::BTreeMap;

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
    DATA_TOML,
};

lazy_static! {
    #[derive(Debug)]
    pub static ref GENERATORS: Mutex<BTreeMap<String, Generator>> = {
        Mutex::new(BTreeMap::new())
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorBuildConfig {
    // Destination output for generated pages
    // relative to the site
    pub destination: String,
    // Name of the template used for page generation
    pub template: String,
    // Name of an index file to copy to the generated directory
    pub index: Option<String>,
    // Whether to output a JSON file containing the data
    pub json: Option<String>,
    // Whether the index template is passed a `documents` array
    pub include_documents: bool,
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

        if let Some(ind) = &self.index {
            let mut i = f.to_path_buf();
            i.push(ind);
            if !i.exists() || !i.is_file() {
                return Err(
                    Error::new(
                        format!("Generator index '{}' is not a file", ind)))
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorConfig {
    pub build: GeneratorBuildConfig,
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
    pub documents: Vec<SourceDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceDocument {
    pub id: String,
    pub value: Value,
}

impl Generator {
    pub fn load(&mut self) -> Result<(), Error> {
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
                            let value: Value =
                                serde_json::from_str(&contents)?;
                            if let Some(stem) = path.file_stem() {
                                let id = stem.to_string_lossy().into_owned();
                                self.documents.push(SourceDocument{id, value});
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

fn load_documents() -> Result<(), Error> {
    let mut generators = GENERATORS.lock().unwrap();
    for (k, g) in generators.iter_mut() {
        g.load()?;
        info!("{} < {}", k, g.source.display());
    }
    Ok(())
}

fn load_configurations(opts: &BuildOptions) -> Result<(), Error> {
    let mut generators = GENERATORS.lock().unwrap();
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
                                conf.push(DATA_TOML);
                                if !conf.exists() || !conf.is_file() {
                                    return Err(
                                        Error::new(
                                            format!("No {} for generator {}", DATA_TOML, key)));
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

                                let generator = Generator {
                                    site: opts.source.clone(),
                                    source: path.to_path_buf(),
                                    documents: Vec::new(),
                                    config,
                                };

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

pub fn load(opts: &BuildOptions) -> Result<(), Error> {
    load_configurations(opts)?;
    load_documents()
}
