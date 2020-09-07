use std::path::PathBuf;
use std::sync::Arc;

use log::warn;

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use collator::LayoutCollate;
use config::Page;
use locale::{Locales, LOCALES};

use crate::{Error, Result};

use super::context::BuildContext;
use super::helpers;

static TEMPLATE_EXT: &str = ".hbs";

// Render templates using handlebars.
#[derive(Debug)]
pub struct Parser<'a> {
    context: Arc<BuildContext>,
    handlebars: Handlebars<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(
        context: Arc<BuildContext>,
        locales: &'a Locales,
    ) -> Result<Self> {
        let mut handlebars = Handlebars::new();

        let settings = &context.options.settings;
        let strict = settings.strict.is_some() && settings.strict.unwrap();
        handlebars.set_strict_mode(strict);

        // Register short code directories
        if context.options.settings.should_use_short_codes() {
            let short_codes = config::get_short_codes_location()?;
            if short_codes.exists() && short_codes.is_dir() {
                handlebars
                    .register_templates_directory(TEMPLATE_EXT, &short_codes)?;
            } else {
                warn!("Short codes are enabled but the short code cache does not exist.");
                warn!("Use the `fetch` command to download the short codes repository.");
                return Err(Error::NoShortCodeCache(short_codes));
            }
        }

        // Built-in partials
        handlebars.register_template_string(
            "charset",
            include_str!("builtins/charset.hbs"),
        )?;
        handlebars.register_template_string(
            "title",
            include_str!("builtins/title.hbs"),
        )?;
        handlebars.register_template_string(
            "viewport",
            include_str!("builtins/viewport.hbs"),
        )?;
        handlebars.register_template_string(
            "edge",
            include_str!("builtins/edge.hbs"),
        )?;
        handlebars.register_template_string(
            "description",
            include_str!("builtins/description.hbs"),
        )?;
        handlebars.register_template_string(
            "keywords",
            include_str!("builtins/keywords.hbs"),
        )?;
        handlebars.register_template_string(
            "canonical",
            include_str!("builtins/canonical.hbs"),
        )?;
        handlebars.register_template_string(
            "noindex",
            include_str!("builtins/noindex.hbs"),
        )?;
        handlebars.register_template_string(
            "head",
            include_str!("builtins/head.hbs"),
        )?;

        // Configure partial directories
        let templates = context.options.get_partials_path();
        if templates.exists() && templates.is_dir() {
            handlebars
                .register_templates_directory(TEMPLATE_EXT, &templates)?;
        }

        // Configure helpers
        handlebars.register_helper(
            "author",
            Box::new(helpers::author::AuthorMeta { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "partial",
            Box::new(helpers::partial::Partial { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "children",
            Box::new(helpers::children::Children { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "livereload",
            Box::new(helpers::livereload::LiveReload { context: Arc::clone(&context) }),
        );
        handlebars
            .register_helper("feed", Box::new(helpers::feed::Feed { context: Arc::clone(&context) }));
        handlebars.register_helper(
            "parent",
            Box::new(helpers::parent::Parent { context: Arc::clone(&context) }),
        );
        handlebars
            .register_helper("link", Box::new(helpers::link::Link { context: Arc::clone(&context) }));
        handlebars.register_helper(
            "md",
            Box::new(helpers::markdown::Markdown { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "components",
            Box::new(helpers::components::Components { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "match",
            Box::new(helpers::matcher::Match { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "series",
            Box::new(helpers::series::Series { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "favicon",
            Box::new(helpers::favicon::Icon { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "bookmark",
            Box::new(helpers::bookmark::Link { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "permalink",
            Box::new(helpers::bookmark::PermaLink { context: Arc::clone(&context) }),
        );

        handlebars.register_helper(
            "styles",
            Box::new(helpers::styles::Styles { context: Arc::clone(&context) }),
        );
        handlebars.register_helper(
            "scripts",
            Box::new(helpers::scripts::Scripts { context: Arc::clone(&context) }),
        );

        if context.config.search.is_some() {
            handlebars.register_helper(
                "search",
                Box::new(helpers::search::Embed { context: Arc::clone(&context) }),
            );
        }

        handlebars.register_helper("json", Box::new(helpers::json::Debug));
        handlebars
            .register_helper("include", Box::new(helpers::include::Include));
        handlebars.register_helper("random", Box::new(helpers::random::Random));
        handlebars.register_helper("slug", Box::new(helpers::slug::Slug));
        handlebars.register_helper("date", Box::new(helpers::date::DateFormat));

        handlebars.register_helper(
            "next",
            Box::new(helpers::sibling::Sibling {
                name: String::from("next"),
                amount: 1,
            }),
        );
        handlebars.register_helper(
            "previous",
            Box::new(helpers::sibling::Sibling {
                name: String::from("previous"),
                amount: -1,
            }),
        );

        if let Some(loader) = &locales.loader.arc {
            handlebars.register_helper(
                "fluent",
                Box::new(FluentLoader::new(loader.as_ref())),
            );
        } else {
            handlebars.register_helper(
                "fluent",
                Box::new(FluentLoader::new(&*LOCALES)),
            );
        }

        // Conditional helpers
        if let Some(ref transform) = context.config.transform {
            if let Some(ref html) = transform.html {
                if html.use_toc() {
                    handlebars.register_helper(
                        "toc",
                        Box::new(helpers::toc::TableOfContents),
                    );
                }

                if html.use_words() {
                    handlebars.register_helper(
                        "words",
                        Box::new(helpers::word::Count),
                    );
                }
            }
        }

        let layouts = context.collation.layouts();
        for (name, path) in layouts.iter() {
            handlebars.register_template_file(name, path)?;
        }

        Ok(Parser {
            context,
            handlebars,
        })
    }

    fn get_front_matter_config(&self, file: &PathBuf) -> frontmatter::Config {
        if let Some(ext) = file.extension() {
            if ext == config::HTML {
                return frontmatter::Config::new_html(false);
            }
        }
        frontmatter::Config::new_markdown(false)
    }

    fn layout(&self, data: &Page, layout: &PathBuf) -> Result<String> {
        let layout_name = layout.to_string_lossy().into_owned();
        return self
            .handlebars
            .render(&layout_name, data)
            .map_err(Error::from);
    }

    fn standalone(&self, file: &PathBuf, data: &Page) -> Result<String> {
        let (content, _has_fm, _fm) =
            frontmatter::load(file, self.get_front_matter_config(file))?;
        self.handlebars
            .render_template(&content, data)
            .map_err(Error::from)
    }

    pub fn parse(&self, file: &PathBuf, data: &Page) -> Result<String> {
        // Explicitly marked as standalone
        if let Some(standalone) = data.standalone {
            if standalone {
                return self.standalone(file, data);
            }
        }

        let layout = self.context.collation.find_layout(file);
        if let Some(ref layout_path) = layout {
            self.layout(data, layout_path)
        } else {
            self.standalone(file, data)
        }
    }
}
