use crate::config::Config;
use serde::{Deserialize, Serialize};

use super::generator::GeneratorMap;
use crate::locale::Locales;

use super::CompilerOptions;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub config: Config,
    pub options: CompilerOptions,
    #[serde(skip)]
    pub livereload: Option<String>,
    #[serde(skip)]
    pub generators: GeneratorMap,
    #[serde(skip)]
    pub locales: Locales,
}

impl Context {
    pub fn new(
        locales: Locales,
        config: Config,
        options: CompilerOptions,
        generators: GeneratorMap,
    ) -> Self {
        Self {
            locales,
            config,
            options,
            livereload: None,
            generators,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            //lang: String::from("en"),
            config: Default::default(),
            options: Default::default(),
            generators: Default::default(),
            locales: Default::default(),
            livereload: None,
        }
    }
}
