use std::borrow::Cow;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::sync::Arc;

use log::debug;

use bracket::{Registry, template::{Templates, Loader}};
use bracket_fluent::FluentHelper;

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

//#[derive(Debug)]
struct ParserBuilder<'reg, 'source> {
    engine: TemplateEngine,
    context: Arc<BuildContext>,
    registry: Registry<'reg, 'source>,
    loader: Loader,
}

impl<'reg, 'source> ParserBuilder<'reg, 'source> {
    pub fn new(engine: TemplateEngine, context: Arc<BuildContext>) -> Self {
        let mut registry = Registry::new();

        let strict = context.options.settings.strict.is_some()
            && context.options.settings.strict.unwrap();
        registry.set_strict(strict);

        let loader = Loader::new();

        Self {
            engine,
            context,
            loader,
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
                            self.loader.add(nm, partial.to_path_buf(plugin.base()))?;

                            /*
                            self.registry.register_template_file(
                                nm,
                                partial.to_path_buf(plugin.base()),
                            )?;
                            */
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

            self.loader.read_dir(
                &templates,
                self.engine.get_template_extension())?;

            /*
            self.registry.register_templates_directory(
                self.engine.get_template_extension(),
                &templates,
            )?;
            */
        }
        Ok(self)
    }


    pub fn helpers(mut self) -> Result<Self> {
        // Configure helpers
        self.registry.helpers_mut().insert(
            "document",
            Box::new(helpers::document::Document {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "block",
            Box::new(helpers::document::Block {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "render",
            Box::new(helpers::document::RenderPage {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "links",
            Box::new(helpers::links::Links {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "menu",
            Box::new(helpers::menu::Menu {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "feed",
            Box::new(helpers::feed::Feed {
                context: Arc::clone(&self.context),
            }),
        );
        self.registry.helpers_mut().insert(
            "page",
            Box::new(helpers::page::Page {
                context: Arc::clone(&self.context),
            }),
        );
        self.registry.helpers_mut().insert(
            "parent",
            Box::new(helpers::parent::Parent {
                context: Arc::clone(&self.context),
            }),
        );
        self.registry.helpers_mut().insert(
            "link",
            Box::new(helpers::link::Link {
                context: Arc::clone(&self.context),
            }),
        );
        self.registry.helpers_mut().insert(
            "md",
            Box::new(helpers::markdown::Markdown {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "crumbtrail",
            Box::new(helpers::crumbtrail::Crumbtrail {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "match",
            Box::new(helpers::matcher::Match {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "scripts",
            Box::new(helpers::scripts::Scripts {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut().insert(
            "search",
            Box::new(helpers::search::Embed {
                context: Arc::clone(&self.context),
            }),
        );

        self.registry.helpers_mut()
            .insert("include", Box::new(helpers::include::Include));
        self.registry.helpers_mut()
            .insert("random", Box::new(helpers::random::Random));
        self.registry.helpers_mut()
            .insert("slug", Box::new(helpers::slug::Slug));
        self.registry.helpers_mut()
            .insert("date", Box::new(helpers::date::DateFormat));

        self.registry.helpers_mut().insert(
            "next",
            Box::new(helpers::sibling::Sibling {
                name: String::from("next"),
                amount: 1,
            }),
        );
        self.registry.helpers_mut().insert(
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
                    self.registry.helpers_mut().insert(
                        "toc",
                        Box::new(helpers::toc::TableOfContents),
                    );
                }

                if html.use_words() {
                    self.registry.helpers_mut().insert(
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
            self.loader.insert(&name, template);
            //self.registry.register_template_string(&name, template)?;
        }

        drop(collation);

        Ok(self)
    }

    pub fn fluent(mut self, locales: Arc<Locales>) -> Result<Self> {
        let loader = locales.loader();

        if let Some(loader) = loader {

            self.registry
                .helpers_mut()
                .insert("fluent", Box::new(FluentHelper::new(Box::new(loader.as_ref()))));

            //self.registry.register_helper(
                //"fluent",
                //Box::new(FluentLoader::new(loader.as_ref())),
            //);
        } else {
            //self.registry.register_helper(
                //"fluent",
                //Box::new(FluentLoader::new(&*LOCALES)),
            //);

            self.registry
                .helpers_mut()
                .insert("fluent", Box::new(FluentHelper::new(Box::new(&*LOCALES))));

        }

        Ok(self)
    }

    pub fn layouts(mut self) -> Result<Self> {
        let layouts = self.context.collation.read().unwrap().layouts().clone();
        for (name, path) in layouts.iter() {
            debug!("Layout: {}", name);
            self.loader.add(name, path.as_ref())?;
            //self.registry.register_template_file(name, path.as_ref())?;
        }
        Ok(self)
    }

    pub fn build(self) -> Result<BracketParser<'reg, 'source>> {
        let templates = Templates::try_from(&self.loader)?;
        Ok(BracketParser {
            context: self.context,
            loader: self.loader,
            registry: self.registry,
        })
    }
}

// Render templates using handlebars.
//#[derive(Debug)]
pub struct BracketParser<'reg, 'source> {
    context: Arc<BuildContext>,
    loader: Loader,
    registry: Registry<'reg, 'source>,
}

impl Parser for BracketParser<'_, '_> {
    fn parse(
        &self,
        _file: &PathBuf,
        data: CollatedPage,
    ) -> Result<String> {
        self
            .registry
            .render(DOCUMENT, &data)
            .map_err(Error::from)
    }
}
