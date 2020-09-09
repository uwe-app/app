use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not resolve page data for file {0}")]
    PageResolve(PathBuf),

    #[error("Invalid resource operation attempted on {0}")]
    InvalidResourceOperation(PathBuf),

    #[error("Parser got invalid file type")]
    ParserFileType,

    #[error("Short code cache is not a directory {0}")]
    NoShortCodeCache(PathBuf),

    #[error("Failed to get canonical path for project root {0}")]
    CanonicalProjectRoot(PathBuf),

    #[error("Path parameter for listing is not a directory {0}")]
    ListingNotDirectory(PathBuf),

    #[error("Resources not a directory {0}")]
    ResourceNotDirectory(PathBuf),

    #[error("Multiple build errors")]
    Multi { errs: Vec<Error> },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    //#[error(transparent)]
    //InvalidUri(#[from] http::uri::InvalidUri),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

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
    Config(#[from] config::Error),
    #[error(transparent)]
    Collator(#[from] collator::Error),
    #[error(transparent)]
    Transform(#[from] transform::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub mod build;
pub mod context;
pub mod helpers;
pub mod hook;
pub mod lookup;
pub mod markdown;
pub mod parser;
pub mod run;
pub mod tree;

pub use build::Compiler;
pub use context::*;
pub use run::ParseData;
