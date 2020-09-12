use std::path::PathBuf;
use std::sync::Arc;

use log::warn;

use serde::Serialize;

use fluent_templates::FluentLoader;
use handlebars::Handlebars;

use collator::LayoutCollate;
use locale::{Locales, LOCALES};

use crate::{Error, Result};

use config::CollatedPage;

use super::context::BuildContext;
use super::helpers;

static TEMPLATE_EXT: &str = ".hbs";

pub trait Parser {
    fn parse(
        &self,
        file: &PathBuf,
        // NOTE: we would like to use `impl Serialize` here
        // NOTE: but cannot due to E0038
        data: CollatedPage,
        standalone: bool,
    ) -> Result<String>;
}

/// Generate the standard parser.
pub fn handlebars<'a>(
    context: Arc<BuildContext>,
    locales: Arc<Locales>,
) -> Result<Box<impl Parser + Send + Sync + 'a>> {
    let builder = ParserBuilder::new(context)
        .short_codes()?
        .builtins()?
        .partials()?
        .helpers()?
        .fluent(locales)?
        .layouts()?;
    Ok(Box::new(builder.build()?))
}

#[derive(Debug)]
struct ParserBuilder<'a> {
    context: Arc<BuildContext>,
    handlebars: Handlebars<'a>,
}

impl<'a> ParserBuilder<'a> {
    pub fn new(context: Arc<BuildContext>) -> Self {
        let mut handlebars = Handlebars::new();

        let strict = context.options.settings.strict.is_some()
            && context.options.settings.strict.unwrap();
        handlebars.set_strict_mode(strict);

        Self {
            context,
            handlebars,
        }
    }

    pub fn short_codes(mut self) -> Result<Self> {
        // Register short code directories
        if self.context.options.settings.should_use_short_codes() {
            let short_codes = config::get_short_codes_location()?;
            if short_codes.exists() && short_codes.is_dir() {
                self.handlebars
                    .register_templates_directory(TEMPLATE_EXT, &short_codes)?;
            } else {
                warn!("Short codes are enabled but the short code cache does not exist.");
                warn!("Use the `fetch` command to download the short codes repository.");
                return Err(Error::NoShortCodeCache(short_codes));
            }
        }

        Ok(self)
    }

    pub fn builtins(mut self) -> Result<Self> {
        // Built-in partials
        self.handlebars.register_template_string(
            "charset",
            include_str!("builtins/charset.hbs"),
        )?;
        self.handlebars.register_template_string(
            "title",
            include_str!("builtins/title.hbs"),
        )?;
        self.handlebars.register_template_string(
            "viewport",
            include_str!("builtins/viewport.hbs"),
        )?;
        self.handlebars.register_template_string(
            "edge",
            include_str!("builtins/edge.hbs"),
        )?;
        self.handlebars.register_template_string(
            "description",
            include_str!("builtins/description.hbs"),
        )?;
        self.handlebars.register_template_string(
            "keywords",
            include_str!("builtins/keywords.hbs"),
        )?;
        self.handlebars.register_template_string(
            "canonical",
            include_str!("builtins/canonical.hbs"),
        )?;
        self.handlebars.register_template_string(
            "noindex",
            include_str!("builtins/noindex.hbs"),
        )?;
        self.handlebars.register_template_string(
            "head",
            include_str!("builtins/head.hbs"),
        )?;

        Ok(self)
    }

    pub fn partials(mut self) -> Result<Self> {
        // Configure partial directories
        let templates = self.context.options.get_partials_path();
        if templates.exists() && templates.is_dir() {
            self.handlebars
                .register_templates_directory(TEMPLATE_EXT, &templates)?;
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
            "children",
            Box::new(helpers::children::Children {
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
            "series",
            Box::new(helpers::series::Series {
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

        if self.context.config.search.is_some() {
            self.handlebars.register_helper(
                "search",
                Box::new(helpers::search::Embed {
                    context: Arc::clone(&self.context),
                }),
            );
        }

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
        self.handlebars
            .render_template(&content, &data)
            .map_err(Error::from)
    }
}

impl Parser for HandlebarsParser<'_> {
    fn parse(
        &self,
        file: &PathBuf,
        data: CollatedPage,
        standalone: bool,
    ) -> Result<String> {
        if standalone {
            return self.standalone(file, data);
        }
        let collation = &*self.context.collation.read().unwrap();
        let layout = collation.find_layout(file);
        if let Some(ref layout_path) = layout {
            self.layout(data, layout_path)
        } else {
            self.standalone(file, data)
        }
    }
}
