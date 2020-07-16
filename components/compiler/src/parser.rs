use std::path::PathBuf;

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use log::warn;

use config::{Page};

use crate::{Error, Result, TEMPLATE_EXT};

use super::context::BuildContext;
use super::helpers;

// Render templates using handlebars.
pub struct Parser<'a> {
    context: &'a BuildContext,
    handlebars: Handlebars<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(context: &'a BuildContext) -> Result<Self> {
        let mut handlebars = Handlebars::new();

        let settings = &context.options.settings;
        let strict = settings.strict.is_some() && settings.strict.unwrap();
        handlebars.set_strict_mode(strict);

        let templates = context.options.get_partials_path();
        if templates.exists() && templates.is_dir() {
            handlebars.register_templates_directory(TEMPLATE_EXT, &templates)?;
        }
        if context.options.settings.should_use_short_codes() {
            let short_codes = context.options.get_short_codes_path();
            if short_codes.exists() && short_codes.is_dir() {
                handlebars.register_templates_directory(TEMPLATE_EXT, &short_codes)?;
            }
        }

        handlebars.register_helper("partial",
            Box::new(helpers::partial::Partial { context }));

        handlebars.register_helper("children",
            Box::new(helpers::children::Children { context }));
        handlebars.register_helper("livereload",
            Box::new(helpers::livereload::LiveReload { context }));
        handlebars.register_helper("parent",
            Box::new(helpers::parent::Parent { context }));
        handlebars.register_helper("link",
            Box::new(helpers::url::Link { context }));
        handlebars.register_helper("md",
            Box::new(helpers::markdown::Markdown { context }));
        handlebars.register_helper("components",
            Box::new(helpers::url::Components { context }));
        handlebars.register_helper("match",
            Box::new(helpers::url::Match { context }));

        handlebars.register_helper("json", Box::new(helpers::json::Debug));
        handlebars.register_helper("include", Box::new(helpers::include::Include));
        handlebars.register_helper("random", Box::new(helpers::random::Random));
        handlebars.register_helper("slug", Box::new(helpers::slug::Slug));
        handlebars.register_helper("date", Box::new(helpers::date::DateFormat));

        if let Some(loader) = &context.locales.loader.arc {
            handlebars.register_helper("fluent", Box::new(FluentLoader::new(loader.as_ref())));
        }

        Ok(Parser { context, handlebars })
    }

    fn resolve(&self, data: &Page) -> Option<PathBuf> {
        // Skip layout for standalone documents
        if let Some(standalone) = data.standalone {
            if standalone { return None }
        }

        // See if the file has a specific layout
        let layout_path = if let Some(layout) = &data.layout {
            let mut layout_path = self.context.options.source.clone();
            layout_path.push(layout);
            if !layout_path.exists() {
                warn!("Missing layout {}", layout_path.display());
            }
            layout_path
        } else {
            // Respect the settings for a build profile
            if let Some(ref layout) = self.context.options.settings.layout {
                layout.clone()
            } else {
                PathBuf::from(config::LAYOUT_HBS)
            }
        };

        if layout_path.exists() {
            return Some(layout_path);
        }

        None
    }

    fn layout(
        &mut self, 
        _file: &PathBuf, data: &Page, layout: &PathBuf) -> Result<String> {

        let layout_name = layout.to_string_lossy().into_owned();
        if !self.handlebars.has_template(&layout_name) {
            if let Err(e) = self
                .handlebars
                .register_template_file(&layout_name, &layout)
            {
                return Err(Error::from(e));
            }
        }
        return self.handlebars.render(&layout_name, data).map_err(Error::from)
    }

    fn get_front_matter_config(&self, file: &PathBuf) -> frontmatter::Config {
        if let Some(ext) = file.extension() {
            if ext == config::HTML {
                return frontmatter::Config::new_html(false)
            } 
        }
        frontmatter::Config::new_markdown(false)
    }

    pub fn parse(&mut self, file: &PathBuf, data: &Page) -> Result<String> {
        let layout = self.resolve(data);
        if let Some(ref layout_path) = layout {
            self.layout(file, data, layout_path)
        } else {
            let (content, _has_fm, _fm) =
                frontmatter::load(file, self.get_front_matter_config(file))?;
            self.handlebars.render_template(&content, data).map_err(Error::from)
        }
    }
}
