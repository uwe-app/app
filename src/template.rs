use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;

use toml::Value;
use toml::de::{Error as TomlError};
use serde_derive::Deserialize;
use inflector::Inflector;

use handlebars::Handlebars;

use log::{info, error};

use super::fs;

const INDEX_STEM: &'static str = "index";

#[derive(Deserialize,Debug)]
struct FileProperties {
    title: Option<String>,
}

/// Manages the data associated with a template.
pub struct DataLoader;

impl DataLoader {
    pub fn new() -> Self {
        DataLoader{}
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

// Render templates using handlebars.
pub struct TemplateRender<'a> {
    layout_name: String,
    pub handlebars: Handlebars<'a>,
}

impl TemplateRender<'_> {
    pub fn new(layout_name: String) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        TemplateRender{layout_name, handlebars}
    }

    fn resolve_layout(&self, input: &PathBuf) -> Option<PathBuf> {
        if let Some(p) = input.parent() {
            // Note that ancestors() does not implement DoubleEndedIterator
            // so we cannot call rev()
            let mut ancestors = p.ancestors().collect::<Vec<_>>();
            ancestors.reverse();
            for p in ancestors {
                let mut copy = p.to_path_buf().clone();
                copy.push(&self.layout_name);
                if copy.exists() {
                    return Some(copy)
                }
            }
        }
        None
    }

    pub fn parse_template(
        &mut self,
        input: &PathBuf,
        content: String,
        data: &mut BTreeMap<&str, Value>) -> io::Result<String> {

        let name = &input.to_str().unwrap();
        // FIXME: call register_template_file
        if self.handlebars.register_template_string(name, &content).is_ok() {

            let filepath = input.to_str().unwrap().to_string();
            data.insert("filepath", Value::String(filepath));

            //println!("render with name {}", name);

            let parsed = self.handlebars.render(name, data);
            match parsed {
                Ok(s) => {
                    return Ok(s)                
                },
                Err(e) => {
                    error!("{}", e);
                }
            }
        }
        Ok(content)
    }

    pub fn layout(
        &mut self,
        input: &PathBuf,
        result: String, data:
        &mut BTreeMap<&str, Value>) -> io::Result<String> {
        if let Some(template) = self.resolve_layout(&input) {
            // Read the layout template
            let template_content = fs::read_string(&template)?;
            // Inject the result into the layout template data
            // re-using the same data object
            data.insert("content", Value::String(result));
            return self.parse_template(&template, template_content, data)
        }
        Ok(result)
    }

}
