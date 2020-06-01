use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use log::{info};

use crate::{
    utils,
    Error,
    BuildOptions,
    TEMPLATE,
    GENERATOR,
    DATA,
    DATA_TOML,
};

lazy_static! {
    #[derive(Debug)]
    pub static ref GENERATORS: Mutex<BTreeMap<String, Generator>> = {
        Mutex::new(BTreeMap::new())
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorConfig {
    pub destination: String,
    pub template: String,
    pub index: Option<String>,
    pub json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Generator {
    pub source: PathBuf,
    pub config: GeneratorConfig,
}

impl GeneratorConfig {
    pub fn validate<P: AsRef<Path>>(&self, dir: P) -> Option<Error> {
        let f = dir.as_ref();

        let mut t = f.to_path_buf();
        t.push(&self.template);
        if !t.exists() || !t.is_file() {
            return Some(
                Error::new(
                    format!("Generator template '{}' is not a file", self.template)))
        }

        let dest = Path::new(&self.destination);
        if dest.is_absolute() {
            return Some(
                Error::new(
                    format!("Generator destination '{}' must be relative path", self.destination)))
        }

        if let Some(ind) = &self.index {
            let mut i = f.to_path_buf();
            i.push(ind);
            if !i.exists() || !i.is_file() {
                return Some(
                    Error::new(
                        format!("Generator index '{}' is not a file", ind)))
            }
        }

        None
    }
}

pub fn load(opts: &BuildOptions) -> Result<(), Error> {
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
                                data.push(DATA);
                                if !data.exists() || !data.is_dir() {
                                    return Err(
                                        Error::new(
                                            format!("No {} directory for generator {}", DATA, key)));
                                }

                                let contents = utils::read_string(conf)?;
                                let config: GeneratorConfig = toml::from_str(&contents)?;

                                if let Some(e) = config.validate(path) {
                                    return Err(e) 
                                }

                                let generator = Generator {
                                    source: opts.source.clone(),
                                    config,
                                };

                                info!("{} < {}", key, data.display());

                                println!("{:?}", generator);
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

