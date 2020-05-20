use std::convert::AsRef;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use handlebars::{Handlebars, TemplateFileError};

use log::{debug, error};

use super::{helpers, utils, Options, LAYOUT_HBS};

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
        handlebars.register_helper("html", Box::new(helpers::html::Element));
        handlebars.register_helper("json", Box::new(helpers::json::Debug));

        TemplateRender {
            options,
            handlebars,
        }
    }

    pub fn register_templates_directory<P: AsRef<Path>>(
        &mut self,
        ext: &'static str,
        dir: P,
    ) -> Result<(), TemplateFileError> {
        self.handlebars.register_templates_directory(ext, dir)
    }

    pub fn parse_template_string(
        &mut self,
        input: &PathBuf,
        content: String,
        data: &mut Map<String, Value>,
    ) -> io::Result<String> {
        let name = &input.to_str().unwrap();
        if self
            .handlebars
            .register_template_string(name, &content)
            .is_ok()
        {
            let filepath = input.to_str().unwrap().to_string();
            let mut ctx: Map<String, Value> = Map::new();
            ctx.insert("file".to_string(), json!(filepath));
            ctx.insert("options".to_string(), json!(self.options));
            data.insert("context".to_string(), json!(ctx));

            debug!("{:?}", data);

            let parsed = self.handlebars.render(name, data);
            match parsed {
                Ok(s) => return Ok(s),
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
        document: String,
        data: &mut Map<String, Value>,
    ) -> io::Result<String> {
        // Skip layout for standalone documents
        if let Some(val) = data.get("standalone") {
            if let Some(_) = val.as_bool() {
                return Ok(document);
            }
        }

        if let Some(template) = utils::inherit(&self.options.source, input, LAYOUT_HBS) {
            let name = template.to_string_lossy().into_owned();
            if !self.handlebars.has_template(&name) {
                if let Err(e) = self.handlebars.register_template_file(&name, &template) {
                    return Err(io::Error::new(io::ErrorKind::Other, e));
                }
            }

            // Inject the result into the layout template data
            // re-using the same data object
            data.insert("template".to_owned(), json!(document));

            let parsed = self.handlebars.render(&name, data);
            match parsed {
                Ok(s) => return Ok(s),
                Err(e) => {
                    error!("{}", e);
                    return Err(io::Error::new(io::ErrorKind::Other, e));
                }
            }
        }

        // Could not resolve a layout so pass back the document
        Ok(document)
    }
}
