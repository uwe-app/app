use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use toml::Value;
use toml::de::{Error as TomlError};
use toml::map::Map;
use inflector::Inflector;

use handlebars::{Handlebars, TemplateFileError};

use log::{error, debug};

use super::fs;
use super::Options;

const INDEX_STEM: &'static str = "index";

/// Manages the data associated with a template.
pub struct DataLoader<'a> {
    name: String,
    options: &'a Options,
}

impl<'a> DataLoader<'a> {
    pub fn new(options: &'a Options) -> Self {

        // Derive the layout.toml from the layout.hbs option
        let mut nm = Path::new(&options.layout).to_path_buf();
        nm.set_extension("toml");
        let name = nm.file_name().unwrap().to_string_lossy().into_owned();

        DataLoader{name, options}
    }

    pub fn create() -> Map<String, Value> {
        Map::new()
    }

    // Convert a file name to title case
    fn file_auto_title<P : AsRef<Path>>(&self, input: P) -> Option<String> {
        let i = input.as_ref();
        if let Some(nm) = i.file_stem() {
            // If the file is an index file, try to get the name 
            // from a parent directory
            if nm == INDEX_STEM {
                if let Some(p) = i.parent() {
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

    fn auto_title<P : AsRef<Path>>(&self, input: P, data: &mut Map<String, Value>) {
        if let Some(auto) = self.file_auto_title(&input.as_ref()) {
            data.insert("title".to_string(), Value::String(auto));
        }
    }

    fn load_file<P : AsRef<Path>>(&self, file: P, data: &mut Map<String, Value>) {
        let src = file.as_ref();
        debug!("toml {}", src.display());
        let properties = fs::read_string(src);
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

    fn load_config(&self, input: &PathBuf, data: &mut Map<String, Value>) {

        if let Some(cfg) = fs::inherit(&self.options.source, input, &self.name) {
            self.load_file(&cfg, data);
        }
    }

    fn load_file_config(&self, input: &PathBuf, data: &mut Map<String, Value>) {
        let mut config = input.clone(); 
        config.set_extension("toml");
        if config.exists() {
            self.load_file(&config, data);
        }
    }

    pub fn load_file_data(&self, input: &PathBuf, data: &mut Map<String, Value>) {
        self.load_config(&input, data);
        self.auto_title(&input, data);
        self.load_file_config(&input, data);
    }
}

// Render templates using handlebars.
pub struct TemplateRender<'a> {
    options: &'a Options,
    handlebars: Handlebars<'a>,
}

impl<'a> TemplateRender<'a> {
    pub fn new(options: &'a Options) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        TemplateRender{options, handlebars}
    }

    pub fn register_templates_directory<P: AsRef<Path>>(&mut self, ext: &'static str, dir: P) 
        -> Result<(), TemplateFileError> {
        self.handlebars.register_templates_directory(ext, dir)
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

        // Skip layout for standalone documents
        if let Some(val) = data.get("standalone") {
            if val.is_bool() && val.as_bool().unwrap() {
                return Ok(document)
            }
        }

        if let Some(template) = fs::inherit(&self.options.source, input, &self.options.layout) {
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
