use std::convert::AsRef;
use std::path::{Path, PathBuf};

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use log::{trace, warn};

use config::{Page, FileInfo};

use super::context::BuildContext;
use super::helpers;
use crate::Error;

// Render templates using handlebars.
pub struct TemplateRender<'a> {
    handlebars: Handlebars<'a>,
}

impl<'a> TemplateRender<'a> {
    pub fn new(context: &'a BuildContext) -> Self {
        let runtime = runtime::runtime().read().unwrap();

        let mut handlebars = Handlebars::new();

        let settings = &runtime.options.settings;
        let strict = settings.strict.is_some() && settings.strict.unwrap();
        handlebars.set_strict_mode(strict);

        handlebars.register_helper("json", Box::new(helpers::json::Debug));

        handlebars.register_helper("children", Box::new(helpers::children::Children));
        handlebars.register_helper("livereload", Box::new(helpers::livereload::LiveReload));
        handlebars.register_helper("parent", Box::new(helpers::parent::Parent));
        handlebars.register_helper("include", Box::new(helpers::include::Include));
        handlebars.register_helper("link", Box::new(helpers::url::Link));
        handlebars.register_helper("components", Box::new(helpers::url::Components));
        handlebars.register_helper("match", Box::new(helpers::url::Match));
        handlebars.register_helper("random", Box::new(helpers::random::Random));
        handlebars.register_helper("slug", Box::new(helpers::slug::Slug));
        handlebars.register_helper("date", Box::new(helpers::date::DateFormat));

        handlebars.register_helper("md", Box::new(helpers::markdown::Markdown));

        if let Some(loader) = &context.locales.loader.arc {
            handlebars.register_helper("fluent", Box::new(FluentLoader::new(loader.as_ref())));
        }

        TemplateRender { handlebars }
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

    pub fn parse_template_string(
        &mut self,
        info: &FileInfo,
        content: String,
        data: &mut Page,
    ) -> Result<String, Error> {

        let name = info.file.to_str().unwrap();
        let runtime = runtime::runtime().read().unwrap();
        if self
            .handlebars
            .register_template_string(name, &content)
            .is_ok()
        {
            data.finalize(&runtime.options.lang, info, &runtime.config)?;
            trace!("{:#?}", data);
            return self.handlebars.render(name, data).map_err(Error::from);
        }
        Ok(content)
    }

    pub fn layout(&mut self, document: String, data: &mut Page) -> Result<String, Error> {
        let runtime = runtime::runtime().read().unwrap();

        // Skip layout for standalone documents
        if let Some(standalone) = data.standalone {
            if standalone {
                return Ok(document);
            }
        }

        // FIXME: improve this logic!
        //
        // See if the file has a specific layout
        let layout_path = if let Some(layout) = &data.layout {
            let mut layout_path = runtime.options.source.clone();
            layout_path.push(layout);
            if !layout_path.exists() {
                warn!("Missing layout {}", layout_path.display());
            }
            layout_path
        } else {
            //self.context.options.layout.clone()
            if let Some(ref layout) = runtime.options.settings.layout {
                layout.clone()
            } else {
                PathBuf::from(config::LAYOUT_HBS)
            }
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
