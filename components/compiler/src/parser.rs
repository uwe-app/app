use std::path::PathBuf;
use std::sync::Arc;

use config::engine::TemplateEngine;
use locale::Locales;

use crate::{context::BuildContext, hbs, page::CollatedPage, Result};

/// The trait all template engines must implement.
pub trait Parser {
    /// Parse a template.
    fn parse(
        &self,
        file: &PathBuf,
        // NOTE: we would like to use `impl Serialize` here
        // NOTE: but cannot due to E0038
        data: CollatedPage,
    ) -> Result<String>;

    /// Add or overwrite a named template.
    fn add(&mut self, name: String, file: &PathBuf) -> Result<()>;

    /// Remove a named template.
    fn remove(&mut self, name: &str);

    /// Load a template from a file.
    fn load(&mut self, file: &PathBuf) -> Result<()>;
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
