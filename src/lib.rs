use std::io;
use std::fmt;
use std::error;
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

use handlebars;
use ignore;
use mdbook;
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
static PARSE_EXTENSIONS:[&str; 2] = [HTML, MD];

static BOOK_TOML: &str = "book.toml";
static BOOK_THEME_KEY: &str = "output.html.theme";

#[derive(Debug)]
pub enum Error {
    Message(String),
    IoError(io::Error),
    TemplateFileError(handlebars::TemplateFileError),
    IgnoreError(ignore::Error),
    BookError(mdbook::errors::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Message(ref s) =>  write!(f,"{}", s),
            Error::IoError(ref e) => e.fmt(f),
            Error::TemplateFileError(ref e) => e.fmt(f),
            Error::IgnoreError(ref e) => e.fmt(f),
            Error::BookError(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::IoError(ref e) => Some(e),
            Error::TemplateFileError(ref e) => Some(e),
            Error::IgnoreError(ref e) => Some(e),
            Error::BookError(ref e) => Some(e),
            _ => None
        }
    }
}

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

