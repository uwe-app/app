use serde::{Deserialize, Serialize};
use crate::command::build::BuildOptions;
use crate::config::Config;

use super::generator::GeneratorMap;
use crate::locale::Locales;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub lang: String,
    pub config: Config,
    pub options: BuildOptions,
    pub livereload: Option<String>,
    #[serde(skip)]
    pub generators: GeneratorMap,
    #[serde(skip)]
    pub locales: Locales,
}

impl Context {
    pub fn new(
        lang: String,
        config: Config,
        options: BuildOptions,
        generators: GeneratorMap) -> Self {

        let locales: Locales = Default::default();

        Self {
            lang,
            config,
            options,
            livereload: None,
            generators,
            locales,
        }

    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            lang: String::from("en"),
            config: Default::default(),
            options: Default::default(),
            generators: Default::default(),
            locales: Default::default(),
            livereload: None,
        } 
    }
}
