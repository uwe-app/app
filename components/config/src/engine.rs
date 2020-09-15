use serde::{Serialize, Deserialize};

/// The supported template engines.
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
