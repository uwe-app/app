#[macro_use]
extern crate lazy_static;

use std::path::PathBuf;

mod build;
mod error;
mod helpers;
mod loader;
mod matcher;
mod parser;
mod minify;
mod template;
mod tree;
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
static DATA_TOML: &str = "data.toml";
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
pub enum BuildTag {
    Custom(String),
    Debug,
    Release
}

impl BuildTag {
    pub fn get_path_name(&self) -> String {
        match self {
            BuildTag::Debug => return "debug".to_owned(),
            BuildTag::Release => return "release".to_owned(),
            BuildTag::Custom(s) => return s.to_owned()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Options {
    pub source: PathBuf,
    pub output: PathBuf,
    pub target: PathBuf,
    pub release: bool,
    pub follow_links: bool,
    pub strict: bool,
    pub clean_url: bool,
    pub minify: bool,
    pub tag: BuildTag,
}

pub fn build(options: Options) -> Result<(), Error> {
    if let Err(e) = loader::load(&options) {
        return Err(e)
    }

    //let test = Path::new("site/index.md");
    //println!("{:?}", loader::compute(test));

    let mut builder = Builder::new(&options);
    builder.build()
}
