use config::{Config, RuntimeOptions};
use locale::Locales;

#[derive(Debug, Default)]
pub struct BuildContext {
    pub config: Config,
    pub options: RuntimeOptions,
    pub locales: Locales,
}

impl BuildContext {
    pub fn new(config: Config, options: RuntimeOptions, locales: Locales) -> Self {
        Self { config, options, locales }
    }
}
