use serde::{Deserialize, Serialize};
use std::fmt;

static LAYOUT: &str = "layout";
static HANDLEBARS_EXT: &str = ".hbs";

/// The supported template engines.
///
/// Note that the strings of these enum values returned using
/// to_string() are used to resolve runtime dependencies
/// from cache components and must therefore be safe to use
/// as a file system path component.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TemplateEngine {
    #[serde(rename = "handlebars")]
    Handlebars,
}

impl TemplateEngine {
    pub fn get_template_extension(&self) -> &'static str {
        match *self {
            Self::Handlebars => HANDLEBARS_EXT,
        }
    }

    pub fn get_layout_name(&self) -> String {
        format!("{}{}", LAYOUT, self.get_template_extension())
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::Handlebars
    }
}

impl fmt::Display for TemplateEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Handlebars => write!(f, "{}", "handlebars"),
        }
    }
}
