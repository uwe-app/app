use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use toml::de::Error as TomlError;
use toml::map::Map as TomlMap;
use toml::Value as TomlValue;
use toml::value::Table;

use serde_json::{json, Map, Value};

use log::{warn};

use crate::{
    utils,
    Error,
    MD,
    INDEX_STEM,
    ROOT_TABLE_KEY,
    PARSE_EXTENSIONS
};

use super::frontmatter;

use crate::config::Config;

lazy_static! {
    #[derive(Debug)]
    pub static ref DATA: Mutex<Map<String, Value>> = {
        Mutex::new(Map::new())
    };
}

fn find_file_for_key(k: &str, source: &PathBuf) -> Option<PathBuf> {

    let mut key = k.to_string().clone();
    if k == "/" {
        key = INDEX_STEM.to_string().clone(); 
    } else if key.ends_with("/") {
        key.push_str(INDEX_STEM);
    }

    let mut pth = PathBuf::new();
    pth.push(source);
    pth.push(&key);

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

pub fn table_to_json_map(table: &Table) -> Map<String, Value> {
    let mut map = Map::new();
    for (k, v) in table {
        map.insert(k.to_string(), json!(v));
    }
    map
}

pub fn compute<P: AsRef<Path>>(f: P, frontmatter: bool) -> Result<Map<String, Value>, Error> {
    let mut map: Map<String, Value> = Map::new();
    let data = DATA.lock().unwrap();

    // Look for file specific data
    let file_key = f.as_ref().to_path_buf().to_string_lossy().into_owned();
    let file_object = data.get(&file_key);

    match file_object {
        // Handle returning file specific data, note that
        // these objects have already inherited root properties
        Some(file_object) => {
            if let Some(d) = file_object.as_object() {
                map = d.clone()
            } 
        },
        // Otherwise just return the root object
        None => {
            if let Some(r) = data.get(ROOT_TABLE_KEY) {
                if let Some(r) = r.as_object() {
                    map = r.clone()
                } 
            }
        }
    }

    if let None = map.get("title") {
        if let Some(auto) = utils::file_auto_title(&f) {
            map.insert("title".to_owned(), json!(auto));
        }
    }

    if frontmatter {
        if let Some(ext) = f.as_ref().extension() {
            let conf = if ext == MD {
                frontmatter::Config::new_markdown(true)
            } else {
                frontmatter::Config::new_html(true)
            };
            let (_, has_fm, fm) = frontmatter::load(f.as_ref(), conf)?;
            if has_fm {
                parse_into(fm, &mut map)?;
            }
        }
    }

    Ok(map)
}

pub fn parse_into(source: String, data: &mut Map<String, Value>) -> Result<(), Error> {
    let mut res = parse_toml_to_json(&source)?;
    data.append(&mut res);
    Ok(())
}

pub fn parse_toml_to_json(s: &str) -> Result<Map<String, Value>, Error> {
    let config: TomlMap<String, TomlValue> = toml::from_str(s)?;
    Ok(table_to_json_map(&config))
}

pub fn load_toml_to_json<P: AsRef<Path>>(f: P) -> Result<Map<String, Value>, Error> {
    let res = utils::read_string(f)?;
    parse_toml_to_json(&res)
}

fn clear() {
    let mut data = DATA.lock().unwrap();
    data.clear();
}

pub fn reload(config: &Config, source: &PathBuf) -> Result<(), Error> {
    clear();
    load(config, source)
}

pub fn load(config: &Config, source: &PathBuf) -> Result<(), Error> {
    let src = config.get_data_path(source);

    if src.exists() {
        let mut data = DATA.lock().unwrap();
        let mut root_object = Map::new();

        let properties = utils::read_string(src);
        match properties {
            Ok(s) => {
                let config: Result<TomlMap<String, TomlValue>, TomlError> = toml::from_str(&s);
                match config {
                    Ok(props) => {

                        let root = props.get(ROOT_TABLE_KEY);
                        match root {
                            Some(root) => {
                                let root_table = root.as_table();
                                match root_table {
                                    Some(root) => {
                                        root_object = table_to_json_map(root); 
                                        data.insert(ROOT_TABLE_KEY.to_string(), json!(root_object));
                                    },
                                    None => {
                                        data.insert(ROOT_TABLE_KEY.to_string(), json!(Map::new()));
                                    }
                                }
                            },
                            None => {
                                data.insert(ROOT_TABLE_KEY.to_string(), json!(Map::new()));
                            }
                        }

                        for (k, v) in props {

                            if k == ROOT_TABLE_KEY {
                                continue;
                            }

                            if let Some(props) = v.as_table() {
                                let result = find_file_for_key(&k, source);
                                match result {
                                    Some(f) => {
                                        // Start with the root object properties
                                        let mut file_map = root_object.clone();

                                        // Merge file specific properties
                                        for (k, v) in props {
                                            file_map.insert(k.to_string(), json!(v));
                                        }

                                        // Use the actual file path as the key
                                        // so we can find it easily later
                                        let file_key = f.to_string_lossy().into_owned();
                                        data.insert(file_key, json!(file_map));
                                    },
                                    None => warn!("no file for table: {}", k)
                                }
                            }
                        }
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

