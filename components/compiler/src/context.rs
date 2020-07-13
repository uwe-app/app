use serde::{Deserialize, Serialize};

use config::Config;
use config::RuntimeOptions;

use locale::Locales;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub config: Config,
    pub options: RuntimeOptions,

    #[serde(skip)]
    pub livereload: Option<String>,
    #[serde(skip)]
    pub locales: Locales,
}

impl Context {
    pub fn new(
        locales: Locales,
        config: Config,
        options: RuntimeOptions,
    ) -> Self {
        Self {
            locales,
            config,
            options,
            livereload: None,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            config: Default::default(),
            options: Default::default(),
            locales: Default::default(),
            livereload: None,
        }
    }
}
