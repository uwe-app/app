use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Path {0} is outside the site source")]
    OutsideSourceTree(PathBuf),

    #[error("Data source document should be an object")]
    DataSourceDocumentNotAnObject,

    #[error("Data source document must have an id")]
    DataSourceDocumentNoId,

    #[error("Parser got invalid file type")]
    ParserFileType,

    #[error("Invalidation action not handled")]
    InvalidationActionNotHandled,

    #[error("Short code cache is not a directory {0}")]
    NoShortCodeCache(PathBuf),

    #[error("Failed to get canonical path for project root {0}")]
    CanonicalProjectRoot(PathBuf),

    #[error("Path parameter for listing is not a directory {0}")]
    ListingNotDirectory(PathBuf),

    #[error("Redirect file {0} already exists")]
    RedirectFileExists(PathBuf),

    #[error("Too many redirects, limit is {0}")]
    TooManyRedirects(usize),

    #[error("Cyclic redirect: {stack} <-> {key}")]
    CyclicRedirect{ stack: String, key: String },

    #[error("Resources not a directory {0}")]
    ResourceNotDirectory(PathBuf),

    #[error("Multiple build errors")]
    Multi { errs: Vec<Error> },

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
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    TemplateFile(#[from] handlebars::TemplateFileError),
    #[error(transparent)]
    Template(#[from] handlebars::TemplateError),
    #[error(transparent)]
    TemplateRender(#[from] handlebars::TemplateRenderError),
    #[error(transparent)]
    Render(#[from] handlebars::RenderError),

    #[error(transparent)]
    Book(#[from] book::Error),
    #[error(transparent)]
    DataSource(#[from] datasource::Error),
    #[error(transparent)]
    FrontMatter(#[from] frontmatter::Error),
    #[error(transparent)]
    Loader(#[from] loader::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
}

type Result<T> = std::result::Result<T, Error>;

static TEMPLATE_EXT: &str = ".hbs";
static INDEX_HTML: &str = "index.html";
static INDEX_STEM: &str = "index";
static MD: &str = "md";
static HTML: &str = "html";

pub mod build;
pub mod context;
pub mod draft;
pub mod helpers;
pub mod hook;
pub mod invalidator;
pub mod lookup;
pub mod manifest;
pub mod markdown;
pub mod parser;
pub mod redirect;
pub mod resource;
pub mod run;
pub mod tree;

pub use build::Compiler;
pub use context::BuildContext;
