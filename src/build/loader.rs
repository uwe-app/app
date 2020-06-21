use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::HashMap;

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
    INDEX_STEM
};

use super::frontmatter;
use super::page::Page;

use crate::config::Config;

lazy_static! {
    #[derive(Debug)]
    pub static ref DATA: Mutex<HashMap<String, Page>> = {
        Mutex::new(HashMap::new())
    };
}

fn find_file_for_key(k: &str, source: &PathBuf, config: &Config) -> Option<PathBuf> {

    let mut key = utils::url::to_path_separator(k);
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

    let extensions = &config.extension.as_ref().unwrap();

    // Might just have a file stem so try the
    // supported extensions
    for ext in &extensions.render {
        pth.set_extension(ext);
        if pth.exists() {
            return Some(pth)
        }
    }

    None
}

fn find_key_for_file<P: AsRef<Path>>(f: P, config: &Config) -> String {
    let file = f.as_ref();
    let mut buf = file.to_path_buf();

    if buf.is_dir() {
        let mut tmp = buf.clone();
        tmp.push(INDEX_STEM);  

        let extensions = &config.extension.as_ref().unwrap();
        for ext in &extensions.render {
            tmp.set_extension(ext);
            if tmp.exists() {
                buf = tmp;
                break;
            }
        }
    }

    buf.to_string_lossy().into_owned()
}

pub fn compute<P: AsRef<Path>>(f: P, config: &Config, frontmatter: bool) -> Result<Page, Error> {
    let mut data = DATA.lock().unwrap();

    let mut page = config.page.as_ref().unwrap().clone();

    // Look for file specific data from page.toml
    let file_key = find_key_for_file(&f, config);
    if let Some(file_object) = data.get_mut(&file_key) {
        let mut copy = file_object.clone();
        page.append(&mut copy);
    }

    if let None = page.title {
        if let Some(auto) = utils::file_auto_title(&f) {
            page.title = Some(auto);
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
                parse_into(fm, &mut page)?;
            }
        }

        // FIXME: ensure frontmatter never defines `query`
    }

    Ok(page)
}

pub fn parse_into(source: String, data: &mut Page) -> Result<(), Error> {
    let mut page: Page = toml::from_str(&source)?;
    data.append(&mut page);
    Ok(())
}

pub fn load_toml_to_json<P: AsRef<Path>>(f: P) -> Result<Map<String, Value>, Error> {
    let res = utils::read_string(f)?;
    parse_toml_to_json(&res)
}

pub fn parse_toml_to_json(s: &str) -> Result<Map<String, Value>, Error> {
    let config: TomlMap<String, TomlValue> = toml::from_str(s)?;
    Ok(table_to_json_map(&config))
}

fn table_to_json_map(table: &Table) -> Map<String, Value> {
    let mut map = Map::new();
    for (k, v) in table {
        map.insert(k.to_string(), json!(v));
    }
    map
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
    let src = config.get_page_data_path();
    if src.exists() {
        let mut data = DATA.lock().unwrap();

        let properties = utils::read_string(src);
        match properties {
            Ok(s) => {
                let conf: Result<TomlMap<String, TomlValue>, TomlError> = toml::from_str(&s);
                match conf {
                    Ok(props) => {
                        for (k, v) in props {
                            let page = v.try_into::<Page>()?;
                            let result = find_file_for_key(&k, source, config);
                            match result {
                                Some(f) => {
                                    // Use the actual file path as the key
                                    // so we can find it easily later
                                    let file_key = f.to_string_lossy().into_owned();
                                    //println!("Inserting with key {}", &file_key);
                                    data.insert(file_key, page);
                                },
                                None => warn!("No file for page table: {}", k)
                            }
                        }
                    }
                    Err(e) => return Err(Error::from(e))
                }
            }
            Err(e) => return Err(Error::from(e))
        }
    }
    Ok(())
}

