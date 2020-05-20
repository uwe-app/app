use std::path::PathBuf;

mod build;
mod fs;
mod helpers;
mod loader;
mod matcher;
mod parser;
mod template;
mod utils;

use build::Builder;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Options {
    pub source: PathBuf,
    pub target: PathBuf,
    pub follow_links: bool,
    pub layout: String,
    pub template: String,
    pub theme: String,
    pub clean_url: bool,
    pub minify: bool,
}

pub fn build(options: Options) {
    let mut builder = Builder::new(&options);
    builder.build();
}

