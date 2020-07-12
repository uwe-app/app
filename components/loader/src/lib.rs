#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use toml::de::Error as TomlError;
use toml::map::Map as TomlMap;
use toml::Value as TomlValue;

use inflector::Inflector;

use log::warn;

use thiserror::Error;

use config::{Config, Page, RuntimeOptions};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    FrontMatter(#[from] frontmatter::Error),

    //#[error(transparent)]
    //Link(#[from] link::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
}

static INDEX_STEM: &str = "index";
static MD: &str = "md";

lazy_static! {
    #[derive(Debug)]
    pub static ref DATA: Mutex<HashMap<String, Page>> = {
        Mutex::new(HashMap::new())
    };
}

// Convert a file name to title case
fn file_auto_title<P: AsRef<Path>>(input: P) -> Option<String> {
    let i = input.as_ref();
    if let Some(nm) = i.file_stem() {
        // If the file is an index file, try to get the name
        // from a parent directory
        if nm == INDEX_STEM {
            if let Some(p) = i.parent() {
                return file_auto_title(&p.to_path_buf());
            }
        } else {
            let auto = nm.to_str().unwrap().to_string();
            let capitalized = auto.to_title_case();
            return Some(capitalized);
        }
    }
    None
}

fn find_file_for_key(k: &str, source: &PathBuf, opts: &RuntimeOptions) -> Option<PathBuf> {
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
        return Some(pth);
    }

    //let extensions = &config.extension.as_ref().unwrap();

    if let Some(ref types) = opts.settings.types {
        // Might just have a file stem so try the
        // supported extensions
        for ext in types.render() {
            pth.set_extension(ext);
            if pth.exists() {
                return Some(pth);
            }
        }
    }

    None
}

fn find_key_for_file<P: AsRef<Path>>(f: P, opts: &RuntimeOptions) -> String {
    let file = f.as_ref();
    let mut buf = file.to_path_buf();

    if buf.is_dir() {
        let mut tmp = buf.clone();
        tmp.push(INDEX_STEM);

        //let extensions = &config.extension.as_ref().unwrap();

        if let Some(ref types) = opts.settings.types {
            for ext in types.render() {
                tmp.set_extension(ext);
                if tmp.exists() {
                    buf = tmp;
                    break;
                }
            }
        }

    }

    buf.to_string_lossy().into_owned()
}

pub fn compute<P: AsRef<Path>>(f: P, config: &Config, opts: &RuntimeOptions, frontmatter: bool) -> Result<Page, Error> {
    let mut data = DATA.lock().unwrap();

    let mut page = config.page.as_ref().unwrap().clone();

    // Look for file specific data from page.toml
    let file_key = find_key_for_file(&f, opts);
    if let Some(file_object) = data.get_mut(&file_key) {
        let mut copy = file_object.clone();
        page.append(&mut copy);
    }

    if let None = page.title {
        if let Some(auto) = file_auto_title(&f) {
            page.title = Some(auto);
        }
    }

    if frontmatter {
        if let Some(ext) = f.as_ref().extension() {

            // FIXME: call matcher::get_type() here

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

    page.compute(f, config, opts)?;

    Ok(page)
}

fn parse_into(source: String, data: &mut Page) -> Result<(), Error> {
    let mut page: Page = toml::from_str(&source)?;
    data.append(&mut page);
    Ok(())
}

fn clear() {
    let mut data = DATA.lock().unwrap();
    data.clear();
}

pub fn reload(config: &Config, options: &RuntimeOptions, source: &PathBuf) -> Result<(), Error> {
    clear();
    load(config, options, source)
}

pub fn load(config: &Config, options: &RuntimeOptions, source: &PathBuf) -> Result<(), Error> {
    let src = config.get_page_data_path();
    if src.exists() {
        let mut data = DATA.lock().unwrap();
        let properties = utils::fs::read_string(src)?;
        let conf: Result<TomlMap<String, TomlValue>, TomlError> = toml::from_str(&properties);
        match conf {
            Ok(props) => {
                for (k, v) in props {
                    let page = v.try_into::<Page>()?;
                    let result = find_file_for_key(&k, source, options);
                    match result {
                        Some(f) => {
                            // Use the actual file path as the key
                            // so we can find it easily later
                            let file_key = f.to_string_lossy().into_owned();
                            //println!("Inserting with key {}", &file_key);
                            data.insert(file_key, page);
                        }
                        None => warn!("No file for page table: {}", k),
                    }
                }
            }
            Err(e) => return Err(Error::from(e)),
        }
    }
    Ok(())
}
