use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::generator::Generator;
use crate::command::build::BuildOptions;

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context<'a> {
    pub config: Config,
    pub options: BuildOptions,
    #[serde(skip)]
    pub generators: BTreeMap<String, Generator<'a>>,
    pub livereload: Option<String>,
}

impl<'a> Context<'a> {
    pub fn new(
        config: Config,
        options: BuildOptions) -> Self {
        Context {
            config,
            options,
            generators: BTreeMap::new(),
            livereload: None,
        }

    }
}
