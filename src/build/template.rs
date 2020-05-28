use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use handlebars::Handlebars;

use log::{warn, debug};

use super::helpers;
use crate::{
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

        handlebars.register_helper("children", Box::new(helpers::children::Children));
        handlebars.register_helper("html", Box::new(helpers::html::Element));
        handlebars.register_helper("json", Box::new(helpers::json::Debug));
        handlebars.register_helper("markdown", Box::new(helpers::markdown::Markdown));
        handlebars.register_helper("parent", Box::new(helpers::parent::Parent));
        handlebars.register_helper("include", Box::new(helpers::include::Include));
        handlebars.register_helper("link", Box::new(helpers::url::Link));
        //handlebars.register_helper("page", Box::new(helpers::url::Page));

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

    pub fn parse_template_string<P: AsRef<Path>>(
        &mut self,
        input: P,
        content: String,
        data: &mut Map<String, Value>,
    ) -> Result<String, Error> {
        let name = input.as_ref().to_str().unwrap();
        if self
            .handlebars
            .register_template_string(name, &content)
            .is_ok()
        {
            let filepath = input.as_ref().to_str().unwrap().to_string();
            let mut ctx: Map<String, Value> = Map::new();
            ctx.insert("file".to_string(), json!(filepath));
            ctx.insert("options".to_string(), json!(self.options));

            data.insert("context".to_string(), json!(ctx));

            debug!("{:?}", data);

            return self.handlebars.render(name, data).map_err(Error::from)
        }
        Ok(content)
    }

    pub fn layout<P: AsRef<Path>>(
        &mut self,
        _input: P,
        document: String,
        data: &mut Map<String, Value>,
    ) -> Result<String, Error> {

        // Skip layout for standalone documents
        if let Some(val) = data.get("standalone") {
            if let Some(_) = val.as_bool() {
                return Ok(document);
            }
        }

        // See if the file has a specific layout
        let mut layout_path = PathBuf::new();
        if let Some(path) = data.get("layout") {
            if let Some(name) = path.as_str() {
                layout_path = self.options.source.clone();
                layout_path.push(name);
                if !layout_path.exists() {
                    warn!("missing layout {}", layout_path.display());
                }
            } 
        }

        // Use a default layout path
        if layout_path == PathBuf::new() {
            layout_path = self.options.source.clone();
            layout_path.push(LAYOUT_HBS);
        }

        // No layout available so bail
        if !layout_path.exists() {
            return Ok(document);
        }

        let layout_name = layout_path.to_string_lossy().into_owned();

        if !self.handlebars.has_template(&layout_name) {
            if let Err(e) = self.handlebars.register_template_file(&layout_name, &layout_path) {
                return Err(Error::from(e))
            }
        }

        // Inject the result into the layout template data
        // re-using the same data object
        data.insert("template".to_owned(), json!(document));

        self.handlebars.render(&layout_name, data).map_err(Error::from)
    }
}
