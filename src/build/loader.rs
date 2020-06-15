use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::BTreeMap;

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
    PARSE_EXTENSIONS
};

use super::frontmatter;
use super::page::Page;

use crate::config::Config;

lazy_static! {
    #[derive(Debug)]
    pub static ref DATA: Mutex<BTreeMap<String, Page>> = {
        Mutex::new(BTreeMap::new())
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
    //
    // FIXME: use the extensions config?
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

pub fn compute<P: AsRef<Path>>(f: P, config: &Config, frontmatter: bool) -> Result<Page, Error> {
    let mut data = DATA.lock().unwrap();

    let mut page = config.page.as_ref().unwrap().clone();

    // Look for file specific data
    let file_key = f.as_ref().to_string_lossy().into_owned();
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


    //println!("GOT FILE DATA {:?}", page);

    Ok(page)
}

pub fn parse_into(source: String, data: &mut Page) -> Result<(), Error> {
    //let mut res = parse_toml_to_json(&source)?;
    //data.append(&mut res);
    //

    let mut page: Page = toml::from_str(&source)?;
    data.append(&mut page);
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
    let src = config.get_page_data_path();
    if src.exists() {
        let mut data = DATA.lock().unwrap();

        let properties = utils::read_string(src);
        match properties {
            Ok(s) => {
                let config: Result<TomlMap<String, TomlValue>, TomlError> = toml::from_str(&s);
                match config {
                    Ok(props) => {
                        for (k, v) in props {
                            let page = v.try_into::<Page>()?;
                            let result = find_file_for_key(&k, source);
                            match result {
                                Some(f) => {
                                    // Use the actual file path as the key
                                    // so we can find it easily later
                                    let file_key = f.to_string_lossy().into_owned();
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

