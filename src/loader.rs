use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use toml::de::Error as TomlError;
use toml::map::Map as TomlMap;
use toml::Value as TomlValue;

use serde_json::{json, Map, Value};

use log::{warn};

use super::{utils, Error, Options, PARSE_EXTENSIONS};

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

    // Key already includes a file extension
    if pth.exists() {
        return Some(pth)
    }

    // Might just have a file stem so try the
    // supported extensions
    for ext in &PARSE_EXTENSIONS {
        pth.set_extension(ext);
        if pth.exists() {
            return Some(pth)
        }
    }
    None
}

pub fn compute_into<P: AsRef<Path>>(f: P, map: &mut Map<String, Value>) {
    let data = DATA.lock().unwrap();

    // Get globals first
    let root_object = data.get(ROOT_KEY).unwrap().as_object().unwrap();
    for (k, v) in root_object {
        map.insert(k.to_string(), json!(v));
    }

    if let Some(auto) = utils::file_auto_title(&f) {
        map.insert("title".to_owned(), json!(auto));
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
}

pub fn compute<P: AsRef<Path>>(f: P) -> Map<String, Value> {
    let mut map: Map<String, Value> = Map::new();
    compute_into(f, &mut map);
    map
}

pub fn load(opts: &Options) -> Result<(), Error> {
    let mut src = opts.source.to_path_buf();
    // FIXME: use a constant here
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

