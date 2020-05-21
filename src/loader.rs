use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use toml::de::Error as TomlError;
use toml::map::Map as TomlMap;
use toml::Value as TomlValue;

use serde_json::{json, Map, Value};

use log::{debug, warn};

use super::{utils, Error, Options, LAYOUT_TOML, PARSE_EXTENSIONS};

static ROOT_KEY: &str = "/";

lazy_static! {
    #[derive(Debug)]
    pub static ref DATA: Mutex<Map<String, Value>> = {
        Mutex::new(Map::new())
    };
}

fn get_file_for_key(k: &str, opts: &Options) -> Option<PathBuf> {
    let mut pth = PathBuf::new();
    pth.push(&opts.source);
    pth.push(&k);
    for ext in &PARSE_EXTENSIONS {
        pth.set_extension(ext);
        if pth.exists() {
            return Some(pth)
        }
    }
    None
}

pub fn compute<P: AsRef<Path>>(f: P) -> Map<String, Value> {
    let mut map = Map::new();
    let data = DATA.lock().unwrap();

    // Get globals first
    let root_object = data.get(ROOT_KEY).unwrap().as_object().unwrap();
    for (k, v) in root_object {
        map.insert(k.to_string(), json!(v));
    }

    // Look for file specific data
    let file_key = f.as_ref().to_path_buf().to_string_lossy().into_owned();
    if let Some(d) = data.get(&file_key) {
        if let Some(d) = d.as_object() {
            for (k, v) in d {
                map.insert(k.to_string(), json!(v));
            }
        } 
    }

    map
}

pub fn load(opts: &Options) -> Result<(), Error> {
    let mut src = opts.source.to_path_buf();
    src.push("data.toml");

    if src.exists() {
        let mut data = DATA.lock().unwrap();
        let mut root_object = Map::new();

        let properties = utils::read_string(src);
        match properties {
            Ok(s) => {
                let config: Result<TomlMap<String, TomlValue>, TomlError> = toml::from_str(&s);
                match config {
                    Ok(props) => {
                        for (k, v) in props {
                            if v.is_table() {
                                let result = get_file_for_key(&k, opts);
                                match result {
                                    Some(f) => {
                                        // Use the actual file path as the key
                                        // so we can find it easily later
                                        let file_key = f.to_string_lossy().into_owned();
                                        data.insert(file_key, json!(v));
                                    },
                                    None => warn!("no file for table: {}", k)
                                }
                            } else {
                                root_object.insert(k, json!(v));
                            }
                        }

                        data.insert(ROOT_KEY.to_string(), json!(root_object));
                        //println!("data {:?}", data);

                    }
                    Err(e) => return Err(Error::TomlDeserError(e))
                }
            }
            Err(e) => return Err(Error::IoError(e))
        }
    } else {
        warn!("no data source {}", src.display());
    }

    Ok(())
}

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
