use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to resolve project directory for {0}")]
    ProjectResolve(PathBuf),

    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Missing book configuration {0}")]
    NoBookConfig(PathBuf),

    #[error("No site configuration in {0}")]
    NoSiteConfig(PathBuf),

    #[error("No author found for {0}")]
    NoAuthor(String),

    #[error("Page {0} is outside the source directory {1}")]
    PageOutsideSource(PathBuf, PathBuf),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Lang(#[from] unic_langid::LanguageIdentifierError),
}

type Result<T> = std::result::Result<T, Error>;

mod build;
mod config;
pub mod indexer;
pub mod filter;
pub mod link;
pub mod resolve;
mod page;

pub use config::*;
pub use page::{Page, FileType};
pub use build::{BuildTag, BuildArguments};
pub use indexer::IndexQuery;
