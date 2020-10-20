use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use log::debug;
//use serde::Serialize;

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use collator::{Collate, LayoutCollate};
use locale::{Locales, LOCALES};

use crate::{Error, Result};

use config::engine::TemplateEngine;

use crate::{context::BuildContext, page::CollatedPage, parser::Parser};

static DOCUMENT: &str = "{{document}}";

mod helpers;

/// Generate the standard parser.
pub fn parser<'a>(
    engine: TemplateEngine,
    context: Arc<BuildContext>,
    locales: Arc<Locales>,
) -> Result<Box<impl Parser + Send + Sync + 'a>> {
    let builder = ParserBuilder::new(engine, context)
        .plugins()?
        .partials()?
        .helpers()?
        .menus()?
        .fluent(locales)?
        .layouts()?;
    Ok(Box::new(builder.build()?))
}

#[derive(Debug)]
struct ParserBuilder<'a> {
    engine: TemplateEngine,
    context: Arc<BuildContext>,
    handlebars: Handlebars<'a>,
}

impl<'a> ParserBuilder<'a> {
    pub fn new(engine: TemplateEngine, context: Arc<BuildContext>) -> Self {
        let mut handlebars = Handlebars::new();

        let strict = context.options.settings.strict.is_some()
            && context.options.settings.strict.unwrap();
        handlebars.set_strict_mode(strict);

        Self {
            engine,
            context,
            handlebars,
        }
    }

    /// Register plugin partials.
    pub fn plugins(mut self) -> Result<Self> {
        if let Some(ref cache) = self.context.plugins {
            for (_dep, plugin) in cache.plugins().iter() {
                if let Some(ref templates) =
                    plugin.templates().get(&self.engine)
                {
                    if let Some(ref partials) = templates.partials {
                        for (nm, partial) in partials.iter() {
                            self.handlebars.register_template_file(
                                nm,
                                partial.to_path_buf(plugin.base()),
                            )?;
                        }
                    }
                }
            }
        }
        Ok(self)
    }

    pub fn partials(mut self) -> Result<Self> {
        // Configure partial directories
        let templates = self.context.options.get_partials_path();
        if templates.exists() && templates.is_dir() {
            self.handlebars.register_templates_directory(
                self.engine.get_template_extension(),
                &templates,
            )?;
        }
        Ok(self)
    }

    pub fn helpers(mut self) -> Result<Self> {
        // Configure helpers

        self.handlebars.register_helper(
            "document",
            Box::new(helpers::document::Document {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "block",
            Box::new(helpers::document::Block {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "render",
            Box::new(helpers::document::Render {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "links",
            Box::new(helpers::links::Links {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "menu",
            Box::new(helpers::menu::Menu {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "feed",
            Box::new(helpers::feed::Feed {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "page",
            Box::new(helpers::page::Page {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "parent",
            Box::new(helpers::parent::Parent {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "link",
            Box::new(helpers::link::Link {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "md",
            Box::new(helpers::markdown::Markdown {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "crumbtrail",
            Box::new(helpers::crumbtrail::Components {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "match",
            Box::new(helpers::matcher::Match {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "favicon",
            Box::new(helpers::favicon::Icon {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "bookmark",
            Box::new(helpers::bookmark::Link {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "permalink",
            Box::new(helpers::bookmark::PermaLink {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "scripts",
            Box::new(helpers::scripts::Scripts {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars.register_helper(
            "search",
            Box::new(helpers::search::Embed {
                context: Arc::clone(&self.context),
            }),
        );

        self.handlebars
            .register_helper("json", Box::new(helpers::json::Debug));
        self.handlebars
            .register_helper("include", Box::new(helpers::include::Include));
        self.handlebars
            .register_helper("random", Box::new(helpers::random::Random));
        self.handlebars
            .register_helper("slug", Box::new(helpers::slug::Slug));
        self.handlebars
            .register_helper("date", Box::new(helpers::date::DateFormat));

        self.handlebars.register_helper(
            "next",
            Box::new(helpers::sibling::Sibling {
                name: String::from("next"),
                amount: 1,
            }),
        );
        self.handlebars.register_helper(
            "previous",
            Box::new(helpers::sibling::Sibling {
                name: String::from("previous"),
                amount: -1,
            }),
        );

        // Conditional helpers
        if let Some(ref transform) = self.context.config.transform {
            if let Some(ref html) = transform.html {
                if html.use_toc() {
                    self.handlebars.register_helper(
                        "toc",
                        Box::new(helpers::toc::TableOfContents),
                    );
                }

                if html.use_words() {
                    self.handlebars.register_helper(
                        "words",
                        Box::new(helpers::word::Count),
                    );
                }
            }
        }

        Ok(self)
    }

    pub fn menus(mut self) -> Result<Self> {
        let collation = self.context.collation.read().unwrap();
        let menus = collation.get_graph().get_menus();

        // TODO: register page-specific menu overrides

        for (entry, result) in menus.results() {
            let name = menus.get_menu_template_name(&entry.name);
            let template = Cow::from(&result.value);
            self.handlebars.register_template_string(&name, template)?;
        }

        drop(collation);

        Ok(self)
    }

    pub fn fluent(mut self, locales: Arc<Locales>) -> Result<Self> {
        let loader = locales.loader();
        if let Some(loader) = loader {
            self.handlebars.register_helper(
                "fluent",
                Box::new(FluentLoader::new(loader.as_ref())),
            );
        } else {
            self.handlebars.register_helper(
                "fluent",
                Box::new(FluentLoader::new(&*LOCALES)),
            );
        }

        Ok(self)
    }

    pub fn layouts(mut self) -> Result<Self> {
        let layouts = self.context.collation.read().unwrap().layouts().clone();
        for (name, path) in layouts.iter() {
            debug!("Layout: {}", name);
            self.handlebars.register_template_file(name, path.as_ref())?;
        }
        Ok(self)
    }

    pub fn build(self) -> Result<HandlebarsParser<'a>> {
        Ok(HandlebarsParser {
            context: self.context,
            handlebars: self.handlebars,
        })
    }
}

// Render templates using handlebars.
#[derive(Debug)]
pub struct HandlebarsParser<'a> {
    context: Arc<BuildContext>,
    handlebars: Handlebars<'a>,
}

impl Parser for HandlebarsParser<'_> {
    fn parse(
        &self,
        _file: &PathBuf,
        data: CollatedPage,
    ) -> Result<String> {
        self
            .handlebars
            .render_template(DOCUMENT, &data)
            .map_err(Error::from)
    }
}
