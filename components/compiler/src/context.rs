use serde::{Deserialize, Serialize};

use locale::Locales;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    #[serde(skip)]
    pub livereload: Option<String>,
    #[serde(skip)]
    pub locales: Locales,
}

impl Context {
    pub fn new(locales: Locales) -> Self {
        Self {
            locales,
            livereload: None,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            locales: Default::default(),
            livereload: None,
        }
    }
}
