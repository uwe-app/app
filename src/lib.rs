use std::path::PathBuf;

mod build;
mod fs;
mod helpers;
mod matcher;
mod parser;
mod template;

use build::Builder;

use serde::Serialize;

#[derive(Serialize)]
pub struct Options {
    pub source: PathBuf,
    pub target: PathBuf,
    pub follow_links: bool,
    pub layout: String,
    pub template: String,
    pub theme: String,
    pub clean: bool,
    pub minify: bool,
}

pub fn build(options: Options) {
    let mut builder = Builder::new(&options);
    builder.build();
}

