use std::convert::AsRef;
use std::path::Path;

use serde_json::json;

use handlebars::Handlebars;
use fluent_templates::FluentLoader;

use log::{warn, debug};

use super::page::{Page, FileContext};
use super::context::Context;
use super::helpers;
use crate::{
    Error,
    LAYOUT_HBS
};

// Render templates using handlebars.
pub struct TemplateRender<'a> {
    context: &'a Context,
    handlebars: Handlebars<'a>,
}

impl<'a> TemplateRender<'a> {
    pub fn new(context: &'a Context) -> Self {
        let mut handlebars = Handlebars::new();

        let build = context.config.build.as_ref().unwrap();
        let strict = build.strict.is_some() && build.strict.unwrap();
        handlebars.set_strict_mode(strict);

        handlebars.register_helper("children", Box::new(helpers::children::Children));
        handlebars.register_helper("html", Box::new(helpers::html::Element));
        handlebars.register_helper("json", Box::new(helpers::json::Debug));
        handlebars.register_helper("livereload", Box::new(helpers::livereload::LiveReload));
        handlebars.register_helper("markdown", Box::new(helpers::markdown::Markdown));
        handlebars.register_helper("parent", Box::new(helpers::parent::Parent));
        handlebars.register_helper("include", Box::new(helpers::include::Include));
        handlebars.register_helper("link", Box::new(helpers::url::Link));
        handlebars.register_helper("components", Box::new(helpers::url::Components));
        handlebars.register_helper("match", Box::new(helpers::url::Match));
        handlebars.register_helper("random", Box::new(helpers::random::Random));
        handlebars.register_helper("slug", Box::new(helpers::slug::Slug));
        handlebars.register_helper("date", Box::new(helpers::date::DateFormat));

        if let Some(loader) = &context.locales.loader.arc {
            handlebars.register_helper("fluent", Box::new(FluentLoader::new(loader.as_ref())));
        }

        TemplateRender {
            context,
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
        output: P,
        content: String,
        data: &mut Page,
    ) -> Result<String, Error> {
        let name = input.as_ref().to_str().unwrap();
        if self
            .handlebars
            .register_template_string(name, &content)
            .is_ok()
        {
            let mut file_context = FileContext::new(
                input.as_ref().to_path_buf(),
                output.as_ref().to_path_buf()
            );

            file_context.resolve_metadata()?;

            data.lang = Some(self.context.locales.lang.clone());
            data.file = Some(file_context);

            // Some useful shortcuts
            if let Some(ref date) = self.context.config.date {
                data.vars.insert(
                    "date-formats".to_string(),
                    json!(date.formats));
            }

            // NOTE: context must be pushed into the vars otherwise
            // NOTE: we have a recursive type due to the page data 
            // NOTE: declared in the root config
            data.vars.insert("context".to_string(), json!(self.context));

            debug!("{:?}", data);

            return self.handlebars.render(name, data).map_err(Error::from)
        }
        Ok(content)
    }

    pub fn layout<P: AsRef<Path>>(
        &mut self,
        _input: P,
        _output: P,
        document: String,
        data: &mut Page,
    ) -> Result<String, Error> {

        // Skip layout for standalone documents
        if let Some(standalone) = data.standalone {
            if standalone {
                return Ok(document);
            }
        }

        // See if the file has a specific layout
        let layout_path = if let Some(layout) = &data.layout {
            let mut layout_path = self.context.options.source.clone();
            layout_path.push(layout);
            if !layout_path.exists() {
                warn!("missing layout {}", layout_path.display());
            }
            layout_path
        } else {
            // Use a default layout path
            let mut layout_path = self.context.options.source.clone();
            layout_path.push(LAYOUT_HBS);
            layout_path
        };

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
        data.template = Some(document);

        self.handlebars.render(&layout_name, data).map_err(Error::from)
    }
}
