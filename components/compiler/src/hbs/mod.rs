use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use collator::{Collate, LayoutCollate};
use locale::{Locales, LOCALES};

use crate::{Error, Result};

use config::{markdown, CollatedPage, TemplateEngine};

use crate::context::BuildContext;
use crate::parser::Parser;

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
        if let Some(ref plugins) = self.context.options.plugins {
            for (_name, dep) in plugins.to_vec() {
                let plugin = dep.plugin.as_ref().unwrap();
                if let Some(ref engine_templates) = plugin.templates {
                    if let Some(ref templates) =
                        engine_templates.get(&self.engine)
                    {
                        if let Some(ref partials) = templates.partials {
                            for (nm, partial) in partials.iter() {
                                self.handlebars.register_template_file(
                                    nm,
                                    partial.to_path_buf(&plugin.base),
                                )?;
                            }
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
            "author",
            Box::new(helpers::author::AuthorMeta {
                context: Arc::clone(&self.context),
            }),
        );
        self.handlebars.register_helper(
            "partial",
            Box::new(helpers::partial::Partial {
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
            "livereload",
            Box::new(helpers::livereload::LiveReload {
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
            "components",
            Box::new(helpers::components::Components {
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
            "styles",
            Box::new(helpers::styles::Styles {
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
        let layouts = self.context.collation.read().unwrap().layouts();
        for (name, path) in layouts.iter() {
            self.handlebars.register_template_file(name, path)?;
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

impl<'a> HandlebarsParser<'a> {
    fn get_front_matter_config(&self, file: &PathBuf) -> frontmatter::Config {
        if let Some(ext) = file.extension() {
            if ext == config::HTML {
                return frontmatter::Config::new_html(false);
            }
        }
        frontmatter::Config::new_markdown(false)
    }

    fn layout(&self, data: impl Serialize, layout: &PathBuf) -> Result<String> {
        let layout_name = layout.to_string_lossy().into_owned();
        return self
            .handlebars
            .render(&layout_name, &data)
            .map_err(Error::from);
    }

    fn standalone(
        &self,
        file: &PathBuf,
        data: impl Serialize,
    ) -> Result<String> {
        let (content, _has_fm, _fm) =
            frontmatter::load(file, self.get_front_matter_config(file))?;
        let mut result = self
            .handlebars
            .render_template(&content, &data)
            .map_err(Error::from)?;

        // Normally the `partial` helper will convert to markdown
        // when rendering a page but when standalone we don't expect
        // `partial` to be called and if the page is markdown it is not
        // converted to HTML so this catches that case.
        //
        // This is particularly useful for plugins which ship their
        // own documentation via a standard website but don't want to
        // add any direct dependencies until we have a concept of `dev-dependencies`.
        if self.context.options.is_markdown_file(file) {
            result =
                markdown::render(&mut Cow::from(result), &self.context.config);
        }

        Ok(result)
    }
}

impl Parser for HandlebarsParser<'_> {
    fn parse(
        &self,
        file: &PathBuf,
        data: CollatedPage,
        layout: Option<&PathBuf>,
    ) -> Result<String> {
        if let Some(layout) = layout {
            self.layout(data, layout)
        } else {
            self.standalone(file, data)
        }
    }
}
