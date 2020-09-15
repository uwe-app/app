use std::fmt;
use serde::{Serialize, Deserialize};

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

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::Handlebars
    }
}

impl fmt::Display for TemplateEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Handlebars => {
                write!(f, "{}", "handlebars")
            } 
        }
    }
}
