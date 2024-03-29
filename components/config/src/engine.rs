use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

use crate::Error;

const HANDLEBARS: &str = "handlebars";
const HANDLEBARS_EXT: &str = "hbs";

/// All available template engines.
pub const ENGINES: [TemplateEngine; 1] = [TemplateEngine::Handlebars];

/// The supported template engines.
///
/// Note that the strings of these enum values returned using
/// to_string() are used to resolve runtime dependencies
/// from cache components and must therefore be safe to use
/// as a file system path component.
///
#[derive(Debug, Deserialize, Clone, Hash, PartialEq, Eq)]
pub enum TemplateEngine {
    #[serde(rename = "handlebars")]
    Handlebars,
}

impl TemplateEngine {
    pub fn extension(&self) -> &str {
        match *self {
            Self::Handlebars => HANDLEBARS_EXT,
        }
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
            Self::Handlebars => write!(f, "{}", HANDLEBARS),
        }
    }
}

impl FromStr for TemplateEngine {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == HANDLEBARS {
            return Ok(TemplateEngine::Handlebars);
        }
        Err(Error::UnsupportedTemplateEngine(s.to_string()))
    }
}

impl Serialize for TemplateEngine {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Self::Handlebars => serializer.serialize_str(HANDLEBARS),
        }
    }
}
