use std::path::PathBuf;
use std::collections::BTreeMap;

use toml::Value;
use toml::de::{Error as TomlError};
use serde_derive::Deserialize;
use inflector::Inflector;

use log::{info, error};

use super::fs;

const INDEX_STEM: &'static str = "index";

#[derive(Deserialize,Debug)]
struct FileProperties {
    title: Option<String>,
}

/// Manages the data associated with a template.
pub struct TemplateData;

impl TemplateData {
    pub fn new() -> Self {
        TemplateData{}
    }

    pub fn create() -> BTreeMap<&'static str, Value> {
        BTreeMap::new()
    }

    // Convert a file name to title case
    fn file_auto_title(&self, input: &PathBuf) -> Option<String> {
        if let Some(nm) = input.file_stem() {
            // If the file is an index file, try to get the name 
            // from a parent directory
            if nm == INDEX_STEM {
                if let Some(p) = input.parent() {
                    return self.file_auto_title(&p.to_path_buf());
                }
            } else {
                let auto = nm.to_str().unwrap().to_string();
                let capitalized = auto.to_title_case();
                return Some(capitalized)
            }

        }
        None
    }

    fn auto_title(&self, input: &PathBuf, data: &mut BTreeMap<&str, Value>) {
        if let Some(auto) = self.file_auto_title(&input) {
            data.insert("title", Value::String(auto));
        }
    }

    fn load_file_properties(&self, input: &PathBuf, data: &mut BTreeMap<&str, Value>) {
        let mut props = input.clone(); 
        props.set_extension("toml");
        if props.exists() {
            info!("toml {}", props.display());
            let properties = fs::read_string(&props);
            match properties {
                Ok(s) => {
                    //println!("{}", s);
                    let config: Result<FileProperties, TomlError> = toml::from_str(&s);
                    match config {
                        Ok(props) => {
                            //println!("{:?}", deser);
                            if let Some(title) = props.title {
                                data.insert("title", Value::String(title));
                            }
                        },
                        Err(e) => {
                            error!("{}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("{}", e);
                },
            }
        }
    }

    pub fn load_file_data(&self, input: &PathBuf, data: &mut BTreeMap<&str, Value>) {
        self.auto_title(&input, data);
        self.load_file_properties(&input, data);
    }
}
