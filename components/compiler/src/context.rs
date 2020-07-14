use config::{Config, RuntimeOptions};
use locale::Locales;

#[derive(Debug, Default)]
pub struct Context {
    pub config: Config,
    pub options: RuntimeOptions,
    pub locales: Locales,
}

impl Context {
    pub fn new(config: Config, options: RuntimeOptions, locales: Locales) -> Self {
        Self { config, options, locales }
    }
}

//impl Default for Context {
    //fn default() -> Self {
        //Self { locales: Default::default() }
    //}
//}
