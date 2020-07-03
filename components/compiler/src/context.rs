use serde::{Deserialize, Serialize};

use config::Config;

use locale::Locales;

use super::CompilerOptions;
use datasource::DataSourceMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub config: Config,
    pub options: CompilerOptions,
    #[serde(skip)]
    pub livereload: Option<String>,
    #[serde(skip)]
    pub datasource: DataSourceMap,
    #[serde(skip)]
    pub locales: Locales,
}

impl Context {
    pub fn new(
        locales: Locales,
        config: Config,
        options: CompilerOptions,
        datasource: DataSourceMap,
    ) -> Self {
        Self {
            locales,
            config,
            options,
            livereload: None,
            datasource,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            //lang: String::from("en"),
            config: Default::default(),
            options: Default::default(),
            datasource: Default::default(),
            locales: Default::default(),
            livereload: None,
        }
    }
}
