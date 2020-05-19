use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use toml::Value;
use toml::value::Table;
use toml::de::{Error as TomlError};
use toml::map::Map;

use handlebars::{Handlebars, TemplateFileError};

use log::{error, debug};

use super::fs;
use super::helpers;
use super::Options;
use super::utils;

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

    pub fn create() -> Table {
        Map::new()
    }

    fn load_file<P : AsRef<Path>>(&self, file: P, data: &mut Table) {
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

    fn load_config<P : AsRef<Path>>(&self, input: P, data: &mut Table) {
        if let Some(cfg) = fs::inherit(&self.options.source, &input.as_ref().to_path_buf(), &self.name) {
            self.load_file(&cfg, data);
        }
    }

    fn load_file_config<P : AsRef<Path>>(&self, input: P, data: &mut Table) {
        let mut config = input.as_ref().to_path_buf().clone(); 
        config.set_extension("toml");
        if config.exists() {
            self.load_file(&config, data);
        }
    }

    pub fn load_file_data<P : AsRef<Path>>(&self, input: P, data: &mut Table) {
        self.load_config(&input, data);
        if let Some(auto) = utils::file_auto_title(&input) {
            data.insert("title".to_owned(), Value::String(auto));
        }
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

        handlebars.register_helper("toc", Box::new(helpers::Toc));

        TemplateRender{options, handlebars}
    }

    pub fn register_templates_directory<P: AsRef<Path>>(&mut self, ext: &'static str, dir: P) 
        -> Result<(), TemplateFileError> {
        self.handlebars.register_templates_directory(ext, dir)
    }

    pub fn parse_template_string(&mut self, input: &PathBuf, content: String, data: &mut Table)
        -> io::Result<String> {

        let name = &input.to_str().unwrap();
        if self.handlebars.register_template_string(name, &content).is_ok() {

            let filepath = input.to_str().unwrap().to_string();
            //data.insert("filepath".to_string(), Value::String(filepath));

            let mut ctx: Table = Table::new();
            ctx.insert("file".to_string(), Value::String(filepath));
            ctx.insert(
                "source".to_string(),
                Value::String(self.options.source.to_string_lossy().to_string()));
            ctx.insert(
                "target".to_string(),
                Value::String(self.options.target.to_string_lossy().to_string()));
            ctx.insert(
                "layout".to_string(),
                Value::String(self.options.layout.clone()));
            ctx.insert(
                "clean_url".to_string(),
                Value::Boolean(self.options.clean_url));

            data.insert("context".to_string(), Value::Table(ctx));

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
        &mut self, input: &PathBuf, document: String, data: &mut Table)
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
            data.insert("template".to_string(), Value::String(document));

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

        // Could not resolve a layout so pass back the document
        Ok(document)
    }

}
