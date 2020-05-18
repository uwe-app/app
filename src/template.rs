use std::io;
use std::path::PathBuf;

use toml::Value;
use toml::de::{Error as TomlError};
use toml::map::Map;
use inflector::Inflector;

use handlebars::Handlebars;

use log::{info, error};

use super::fs;

const INDEX_STEM: &'static str = "index";

/// Manages the data associated with a template.
pub struct DataLoader;

impl DataLoader {
    pub fn new() -> Self {
        DataLoader{}
    }

    pub fn create() -> Map<String, Value> {
        Map::new()
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

    fn auto_title(&self, input: &PathBuf, data: &mut Map<String, Value>) {
        if let Some(auto) = self.file_auto_title(&input) {
            data.insert("title".to_string(), Value::String(auto));
        }
    }

    fn load_file_properties(&self, input: &PathBuf, data: &mut Map<String, Value>) {
        let mut props = input.clone(); 
        props.set_extension("toml");
        if props.exists() {
            info!("toml {}", props.display());
            let properties = fs::read_string(&props);
            match properties {
                Ok(s) => {
                    let config: Result<Map<String, Value>, TomlError> = toml::from_str(&s);
                    match config {
                        Ok(props) => {
                            //println!("{:?}", props);
                            for (k, v) in props {
                                data.insert(k, v);
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

    pub fn load_file_data(&self, input: &PathBuf, data: &mut Map<String, Value>) {
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

    pub fn parse_template_string(&mut self, input: &PathBuf, content: String, data: &mut Map<String, Value>)
        -> io::Result<String> {

        let name = &input.to_str().unwrap();
        if self.handlebars.register_template_string(name, &content).is_ok() {

            let filepath = input.to_str().unwrap().to_string();
            data.insert("filepath".to_string(), Value::String(filepath));

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
        &mut self, input: &PathBuf, document: String, data: &mut Map<String, Value>)
        -> io::Result<String> {

        if let Some(template) = self.resolve_layout(&input) {
            let name = template.to_string_lossy().into_owned();
            if !self.handlebars.has_template(&name) {
                if let Err(e) = self.handlebars.register_template_file(&name, &template) {
                    return Err(io::Error::new(io::ErrorKind::Other, e))
                }
            }

            // Inject the result into the layout template data
            // re-using the same data object
            data.insert("content".to_string(), Value::String(document));

            let parsed = self.handlebars.render(&name, data);
            match parsed {
                Ok(s) => {
                    return Ok(s)                
                },
                Err(e) => {
                    error!("{}", e);
                    return Err(io::Error::new(io::ErrorKind::Other, e))
                }
            }
        }

        // Could not resolve a layout to pass back the document
        Ok(document)
    }

}
