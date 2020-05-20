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

static INDEX: &str = "index";
static INDEX_HTML: &str = "index.html";
static TEMPLATE: &str = "template";
static THEME: &str = "theme";
static LAYOUT_HBS: &str = "layout.hbs";
static LAYOUT_TOML: &str = "layout.toml";
static MD: &str = ".md";
static HTML: &str = ".html";
static HBS: &str = ".hbs";
static TOML: &str = ".toml";
static PARSE_EXTENSIONS:[&str; 2] = ["html", "md"];

#[derive(Debug, Serialize, Deserialize)]
pub struct Options {
    pub source: PathBuf,
    pub target: PathBuf,
    pub follow_links: bool,
    pub clean_url: bool,
    pub minify: bool,
}

pub fn build(options: Options) {
    let mut builder = Builder::new(&options);
    builder.build();
}

