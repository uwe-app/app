use serde::{Deserialize, Serialize};
use crate::command::build::BuildOptions;
use crate::config::Config;

use super::generator::GeneratorMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub lang: String,
    pub config: Config,
    pub options: BuildOptions,
    pub livereload: Option<String>,
    #[serde(skip)]
    pub generators: GeneratorMap,
}

impl Context {
    pub fn new(
        lang: String,
        config: Config,
        options: BuildOptions,
        generators: GeneratorMap) -> Self {
        Context {
            lang,
            config,
            options,
            livereload: None,
            generators,
        }

    }
}
