use serde::{Deserialize, Serialize};

use config::Config;

use locale::Locales;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub config: Config,
    #[serde(skip)]
    pub livereload: Option<String>,
    #[serde(skip)]
    pub locales: Locales,
}

impl Context {
    pub fn new(
        locales: Locales,
        config: Config,
    ) -> Self {
        Self {
            locales,
            config,
            livereload: None,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            config: Default::default(),
            locales: Default::default(),
            livereload: None,
        }
    }
}
