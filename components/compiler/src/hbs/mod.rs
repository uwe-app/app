use std::path::PathBuf;
use std::sync::Arc;

use log::debug;

use bracket::Registry;
use bracket_fluent::FluentHelper;

use locale::{Locales, LOCALES};

use crate::{Error, Result};

use config::engine::TemplateEngine;

use crate::{context::BuildContext, page::CollatedPage, parser::Parser};

mod helpers;

/// Generate the standard parser.
pub fn parser<'a>(
    engine: TemplateEngine,
    context: Arc<BuildContext>,
    locales: Arc<Locales>,
) -> Result<Box<impl Parser + Send + Sync + 'a>> {
    let builder = ParserBuilder::new(engine, context)
        .helpers()?
        .fluent(locales)?
        .plugins()?
        .partials()?
        .templates()?
        .menus()?
        .layouts()?;

    Ok(Box::new(builder.build()?))
}

//#[derive(Debug)]
struct ParserBuilder<'reg> {
    engine: TemplateEngine,
    context: Arc<BuildContext>,
    registry: Registry<'reg>,
}

impl<'reg> ParserBuilder<'reg> {
    pub fn new(engine: TemplateEngine, context: Arc<BuildContext>) -> Self {
        let strict = context.options.settings.strict.is_some()
            && context.options.settings.strict.unwrap();

        let mut registry = Registry::new();
        registry.set_strict(strict);

        Self {
            engine,
            context,
            registry,
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
                            self.registry.add(
                                nm.to_string(),
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
            self.registry
                .read_dir(&templates, self.engine.extension())?;
        }
        Ok(self)
    }

    pub fn helpers(mut self) -> Result<Self> {
        // Configure handlers
        self.registry.handlers_mut().link =
            Some(Box::new(helpers::link::WikiLink {
                context: Arc::clone(&self.context),
            }));

        // Configure helpers
        let helpers = self.registry.helpers_mut();

        helpers.insert(
            "document",
            Box::new(helpers::document::Document {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "render",
            Box::new(helpers::document::RenderPage {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "links",
            Box::new(helpers::links::Links {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "menu",
            Box::new(helpers::menu::Menu {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "page",
            Box::new(helpers::page::Page {
                context: Arc::clone(&self.context),
            }),
        );
        helpers.insert(
            "parent",
            Box::new(helpers::parent::Parent {
                context: Arc::clone(&self.context),
            }),
        );
        helpers.insert(
            "powered",
            Box::new(helpers::powered::Powered {
                context: Arc::clone(&self.context),
            }),
        );
        helpers.insert(
            "link",
            Box::new(helpers::link::Link {
                context: Arc::clone(&self.context),
            }),
        );
        helpers.insert(
            "markdown",
            Box::new(helpers::markdown::Markdown {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "crumbtrail",
            Box::new(helpers::crumbtrail::Crumbtrail {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "match",
            Box::new(helpers::matcher::Match {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "scripts",
            Box::new(helpers::scripts::Scripts {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "search",
            Box::new(helpers::search::Embed {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "import",
            Box::new(helpers::import::Import {
                context: Arc::clone(&self.context),
            }),
        );

        helpers.insert(
            "include",
            Box::new(helpers::include::Include {
                context: Arc::clone(&self.context),
            }),
        );
        helpers.insert("random", Box::new(helpers::random::Random));
        helpers.insert("slug", Box::new(helpers::slug::Slug));
        helpers.insert("date", Box::new(helpers::date::DateFormat));
        helpers.insert("bytes", Box::new(helpers::bytes::Bytes));

        helpers.insert(
            "next",
            Box::new(helpers::sibling::Sibling {
                name: String::from("next"),
                amount: 1,
            }),
        );
        helpers.insert(
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
                    helpers
                        .insert("toc", Box::new(helpers::toc::TableOfContents));
                }

                if html.use_words() {
                    helpers.insert("words", Box::new(helpers::word::Count));
                }
            }
        }

        Ok(self)
    }

    /// Register templates in the source tree.
    pub fn templates(mut self) -> Result<Self> {
        let collation = self.context.collation.read().unwrap();
        for path in collation.templates().as_ref() {
            self.registry.load(path.as_ref())?;
        }
        drop(collation);
        Ok(self)
    }

    pub fn menus(mut self) -> Result<Self> {
        let collation = self.context.collation.read().unwrap();

        let menus = collation.get_menus();
        let menus_map = menus.as_ref();

        // TODO: register page-specific menu overrides
        for (key, result) in menus_map {
            let name = collation.get_menu_template_name(key);
            self.registry.insert(&name, &result.value)?;
        }

        drop(menus_map);
        drop(menus);
        drop(collation);

        Ok(self)
    }

    pub fn fluent(mut self, locales: Arc<Locales>) -> Result<Self> {
        let loader = locales.loader();
        if let Some(loader) = loader {
            self.registry.helpers_mut().insert(
                "fluent",
                Box::new(FluentHelper::new(Box::new(loader.as_ref()))),
            );
        } else {
            self.registry.helpers_mut().insert(
                "fluent",
                Box::new(FluentHelper::new(Box::new(&*LOCALES))),
            );
        }

        Ok(self)
    }

    pub fn layouts(mut self) -> Result<Self> {
        let layouts = self.context.collation.read().unwrap().layouts().clone();
        for (name, path) in layouts.iter() {
            debug!("Layout: {}", name);
            self.registry.add(name.to_string(), path.as_ref())?;
        }

        Ok(self)
    }

    pub fn build(self) -> Result<BracketParser<'reg>> {
        Ok(BracketParser {
            //context: self.context,
            registry: self.registry,
        })
    }
}

// Render templates using handlebars.
pub struct BracketParser<'reg> {
    //context: Arc<BuildContext>,
    registry: Registry<'reg>,
}

impl Parser for BracketParser<'_> {
    fn parse(&self, file: &PathBuf, data: CollatedPage) -> Result<String> {
        let name = file.to_string_lossy();

        let standalone = data.page().is_standalone();

        // Try to render a named layout
        if !standalone {
            if let Some(ref layout) = data.page().layout {
                if let Some(_) = self.registry.get(layout) {
                    return self
                        .registry
                        .render(layout, &data)
                        .map_err(Error::from);
                } else {
                    return Err(Error::LayoutNotFound(layout.to_string()));
                }
            }
        }

        // Otherwise just render the page
        let (content, _has_fm, _fm) =
            frontmatter::load(&file, frontmatter::get_config(&file))?;
        return self
            .registry
            .once(&name, content, &data)
            .map_err(Error::from);
    }

    fn add(&mut self, name: String, file: &PathBuf) -> Result<()> {
        self.registry.add(name, file).map_err(Error::from)
    }

    fn remove(&mut self, name: &str) {
        self.registry.remove(name);
    }

    fn load(&mut self, file: &PathBuf) -> Result<()> {
        self.registry.load(file).map_err(Error::from)
    }
}
