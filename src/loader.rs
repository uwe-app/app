use std::convert::AsRef;
use std::path::Path;

use toml::de::Error as TomlError;
use toml::map::Map as TomlMap;
use toml::Value as TomlValue;

use serde_json::{json, Map, Value};

use log::{debug};

use super::{utils, Error, Options, LAYOUT_TOML};

/// Loads the data associated with a template.
pub struct DataLoader<'a> {
    options: &'a Options,
}

impl<'a> DataLoader<'a> {
    pub fn new(options: &'a Options) -> Self {
        DataLoader { options }
    }

    pub fn create() -> Map<String, Value> {
        Map::new()
    }

    fn load_file<P: AsRef<Path>>(&self, file: P, data: &mut Map<String, Value>) -> Result<(), Error> {
        let src = file.as_ref();
        debug!("toml {}", src.display());
        let properties = utils::read_string(src);
        match properties {
            Ok(s) => {
                let config: Result<TomlMap<String, TomlValue>, TomlError> = toml::from_str(&s);
                match config {
                    Ok(props) => {
                        for (k, v) in props {
                            data.insert(k, json!(v));
                        }
                    }
                    Err(e) => return Err(Error::TomlDeserError(e))
                }
            }
            Err(e) => return Err(Error::IoError(e))
        }

        Ok(())
    }

    fn load_config<P: AsRef<Path>>(&self, input: P, data: &mut Map<String, Value>) -> Result<(), Error> {
        // FIXME: this &input handling is wrong!
        if let Some(cfg) = utils::inherit(
            &self.options.source,
            &input.as_ref().to_path_buf(),
            LAYOUT_TOML,
        ) {
            return self.load_file(&cfg, data)
        }
        Ok(())
    }

    fn load_file_config<P: AsRef<Path>>(&self, input: P, data: &mut Map<String, Value>) -> Result<(), Error> {
        let mut config = input.as_ref().to_path_buf().clone();
        config.set_extension("toml");
        if config.exists() {
            return self.load_file(&config, data)
        }
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(&self, input: P, data: &mut Map<String, Value>) -> Result<(), Error> {
        self.load_config(&input, data)?;
        if let Some(auto) = utils::file_auto_title(&input) {
            data.insert("title".to_owned(), Value::String(auto));
        }
        self.load_file_config(&input, data)
    }
}
