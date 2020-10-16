use std::path::PathBuf;
use std::sync::Arc;

use config::{engine::TemplateEngine};
use locale::Locales;

use crate::{context::BuildContext, hbs, Result, page::CollatedPage};

/// The trait all template engines must implement.
pub trait Parser {
    fn parse(
        &self,
        file: &PathBuf,
        // NOTE: we would like to use `impl Serialize` here
        // NOTE: but cannot due to E0038
        data: CollatedPage,
        layout: Option<&PathBuf>,
    ) -> Result<String>;
}

/// Generate a parser for the given template engine.
pub fn build<'a>(
    engine: TemplateEngine,
    context: Arc<BuildContext>,
    locales: Arc<Locales>,
) -> Result<Box<impl Parser + Send + Sync + 'a>> {
    match engine {
        TemplateEngine::Handlebars => hbs::parser(engine, context, locales),
    }
}
