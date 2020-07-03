#[macro_use]
extern crate lazy_static;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    InvalidUri(#[from] warp::http::uri::InvalidUri),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Notify(#[from] notify::Error),
    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    TemplateFile(#[from] handlebars::TemplateFileError),
    #[error(transparent)]
    Render(#[from] handlebars::RenderError),

    #[error(transparent)]
    Book(#[from] book::Error),
    #[error(transparent)]
    DataSource(#[from] datasource::Error),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

type Result<T> = std::result::Result<T, Error>;
pub type ErrorCallback = fn(Error);

static TEMPLATE_EXT: &str = ".hbs";
static INDEX_HTML: &str = "index.html";
static INDEX_STEM: &str = "index";
static MD: &str = "md";
static HTML: &str = "html";

pub mod compiler;
pub mod context;
pub mod draft;
pub mod frontmatter;
pub mod helpers;
pub mod hook;
pub mod invalidator;
pub mod loader;
pub mod manifest;
pub mod markdown;
pub mod matcher;
pub mod parser;
pub mod redirect;
pub mod resource;
pub mod template;
pub mod tree;
mod types;
pub mod watch;

pub use compiler::Compiler;
pub use types::*;
