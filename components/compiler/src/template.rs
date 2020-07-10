use std::convert::AsRef;
use std::path::Path;

use serde_json::json;

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use log::{debug, warn};

use config::Page;

use super::context::Context;
use super::helpers;
use crate::Error;

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
        self.handlebars
            .register_templates_directory(ext, dir)
            .map_err(Error::from)
    }

    pub fn parse_template_string<I: AsRef<Path>, O: AsRef<Path>>(
        &mut self,
        input: I,
        output: O,
        content: String,
        data: &mut Page,
    ) -> Result<String, Error> {
        let name = input.as_ref().to_str().unwrap();
        if self
            .handlebars
            .register_template_string(name, &content)
            .is_ok()
        {
            data.finalize(
                &self.context.locales.lang,
                input.as_ref(),
                output.as_ref(),
                &self.context.config)?;

            // NOTE: context must be pushed into extra otherwise
            // NOTE: we have a recursive type due to the page data
            // NOTE: declared in the root config
            data.extra.insert("context".to_string(), json!(self.context));

            debug!("{:?}", data);

            return self.handlebars.render(name, data).map_err(Error::from);
        }
        Ok(content)
    }

    pub fn layout<I: AsRef<Path>, O: AsRef<Path>>(
        &mut self,
        _input: I,
        _output: O,
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
            self.context.options.layout.clone()
        };

        // No layout available so bail
        if !layout_path.exists() {
            return Ok(document);
        }

        let layout_name = layout_path.to_string_lossy().into_owned();

        if !self.handlebars.has_template(&layout_name) {
            if let Err(e) = self
                .handlebars
                .register_template_file(&layout_name, &layout_path)
            {
                return Err(Error::from(e));
            }
        }

        // Inject the result into the layout template data
        // re-using the same data object
        data.template = Some(document);

        self.handlebars
            .render(&layout_name, data)
            .map_err(Error::from)
    }
}
