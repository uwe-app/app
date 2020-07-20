use std::sync::Arc;
use std::path::PathBuf;

use log::warn;

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use config::{Page};
use locale::Locales;

use crate::{Error, Result};

use super::context::BuildContext;
use super::helpers;

static TEMPLATE_EXT: &str = ".hbs";

// Render templates using handlebars.
pub struct Parser<'a> {
    context: &'a BuildContext,
    handlebars: Handlebars<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(context: &'a BuildContext, locales: &'a Locales) -> Result<Self> {
        let mut handlebars = Handlebars::new();

        let settings = &context.options.settings;
        let strict = settings.strict.is_some() && settings.strict.unwrap();
        handlebars.set_strict_mode(strict);

        // Register short code directories
        if context.options.settings.should_use_short_codes() {
            let short_codes = config::get_short_codes_location()?;
            if short_codes.exists() && short_codes.is_dir() {
                handlebars.register_templates_directory(TEMPLATE_EXT, &short_codes)?;
            } else {
                warn!("Short codes are enabled but the short code cache does not exist.");
                warn!("Use the `fetch` command to download the short codes repository.");
                return Err(Error::NoShortCodeCache(short_codes))
            }
        }

        // Configure partial directories
        let templates = context.options.get_partials_path();
        if templates.exists() && templates.is_dir() {
            handlebars.register_templates_directory(TEMPLATE_EXT, &templates)?;
        }

        // Configure helpers
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

        if let Some(loader) = &locales.loader.arc {
            handlebars.register_helper("fluent", Box::new(FluentLoader::new(loader.as_ref())));
        }

        // Register the global layout
        if let Some(ref l) = context.collation.layout {
            let layout = l.to_path_buf();
            let layout_name = layout.to_string_lossy().into_owned();
            handlebars.register_template_file(&layout_name, &layout)?;
        }

        // Register page-specific layouts
        for (_, l) in context.collation.layouts.iter() {
            let layout = l.to_path_buf();
            let layout_name = layout.to_string_lossy().into_owned();
            handlebars.register_template_file(&layout_name, &layout)?;
        }

        Ok(Parser { context, handlebars })
    }

    fn resolve(&self, file: &PathBuf) -> Option<&PathBuf> {
        if let Some(ref layout) = self.context.collation.layouts.get(&Arc::new(file.to_path_buf())) {
            return Some(layout)
        }
        if let Some(ref layout) = self.context.collation.layout {
            return Some(layout)
        }
        None
    }

    fn get_front_matter_config(&self, file: &PathBuf) -> frontmatter::Config {
        if let Some(ext) = file.extension() {
            if ext == config::HTML {
                return frontmatter::Config::new_html(false)
            } 
        }
        frontmatter::Config::new_markdown(false)
    }

    fn layout(&self, data: &Page, layout: &PathBuf) -> Result<String> {
        let layout_name = layout.to_string_lossy().into_owned();
        return self.handlebars.render(&layout_name, data).map_err(Error::from)
    }

    fn standalone(&self, file: &PathBuf, data: &Page) -> Result<String> {
        let (content, _has_fm, _fm) =
            frontmatter::load(file, self.get_front_matter_config(file))?;
        self.handlebars.render_template(&content, data).map_err(Error::from)
    }

    pub fn parse(&self, file: &PathBuf, data: &Page) -> Result<String> {
        // Explicitly marked as standalone
        if let Some(standalone) = data.standalone {
            if standalone {
                return self.standalone(file, data)
            }
        }

        let layout = self.resolve(file);
        if let Some(ref layout_path) = layout {
            self.layout(data, layout_path)
        } else {
            self.standalone(file, data)
        }
    }
}
