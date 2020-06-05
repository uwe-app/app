use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::generator::Generator;
use crate::command::build::BuildOptions;

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub config: Config,
    pub options: BuildOptions,
    #[serde(skip_serializing)]
    pub generators: BTreeMap<String, Generator>,
    pub livereload: Option<String>,
}

impl Context {
    pub fn new(
        config: Config,
        options: BuildOptions,
        generators: BTreeMap<String, Generator>) -> Self {
        Context {
            config,
            options,
            generators,
            livereload: None,
        }

    }
}
