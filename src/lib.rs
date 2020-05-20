use std::path::PathBuf;

mod build;
mod error;
mod helpers;
mod loader;
mod matcher;
mod parser;
mod template;
mod utils;

use build::Builder;
use serde::{Deserialize, Serialize};

static INDEX_STEM: &str = "index";
static INDEX_HTML: &str = "index.html";
static TEMPLATE: &str = "template";
static TEMPLATE_EXT: &str = ".hbs";
static THEME: &str = "theme";
static LAYOUT_HBS: &str = "layout.hbs";
static LAYOUT_TOML: &str = "layout.toml";
static MD: &str = "md";
static HTML: &str = "html";
static TOML: &str = "toml";
static PARSE_EXTENSIONS: [&str; 2] = [HTML, MD];

static BOOK_TOML: &str = "book.toml";
static BOOK_THEME_KEY: &str = "output.html.theme";

pub use crate::error::Error;

use crate::matcher::FileType;
use crate::parser::Parser;

#[derive(Debug, Serialize, Deserialize)]
pub struct Options {
    pub source: PathBuf,
    pub target: PathBuf,
    pub follow_links: bool,
    pub clean_url: bool,
    pub minify: bool,
}

pub fn build(options: Options) -> Result<(), Error> {
    let mut builder = Builder::new(&options);
    builder.build()
}
