use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use handlebars::Handlebars;

use log::{debug};

use super::helpers;
use crate::{
    utils,
    Error,
    BuildOptions,
    LAYOUT_HBS
};

// Render templates using handlebars.
pub struct TemplateRender<'a> {
    options: &'a BuildOptions,
    handlebars: Handlebars<'a>,
}

impl<'a> TemplateRender<'a> {
    pub fn new(options: &'a BuildOptions) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(options.strict);

        handlebars.register_helper("tree", Box::new(helpers::tree::Tree));
        handlebars.register_helper("html", Box::new(helpers::html::Element));
        handlebars.register_helper("json", Box::new(helpers::json::Debug));
        handlebars.register_helper("markdown", Box::new(helpers::markdown::Markdown));
        handlebars.register_helper("parent", Box::new(helpers::parent::Parent));
        handlebars.register_helper("include", Box::new(helpers::include::Include));

        TemplateRender {
            options,
            handlebars,
        }
    }

    pub fn register_templates_directory<P: AsRef<Path>>(
        &mut self,
        ext: &'static str,
        dir: P,
    ) -> Result<(), Error> {
        self.handlebars.register_templates_directory(ext, dir).map_err(Error::from)
    }

    pub fn parse_template_string(
        &mut self,
        input: &PathBuf,
        content: String,
        data: &mut Map<String, Value>,
    ) -> Result<String, Error> {
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

            let mut url = &"".to_string();
            if let Some(u) = &self.options.livereload {
                url = u
            }

            data.insert("livereload".to_string(), json!(url));
            data.insert("context".to_string(), json!(ctx));

            debug!("{:?}", data);

            return self.handlebars.render(name, data).map_err(Error::from)
        }
        Ok(content)
    }

    pub fn layout(
        &mut self,
        input: &PathBuf,
        document: String,
        data: &mut Map<String, Value>,
    ) -> Result<String, Error> {
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
                    return Err(Error::from(e))
                }
            }

            // Inject the result into the layout template data
            // re-using the same data object
            data.insert("template".to_owned(), json!(document));

            return self.handlebars.render(&name, data).map_err(Error::from)
        }

        // Could not resolve a layout so pass back the document
        Ok(document)
    }
}
