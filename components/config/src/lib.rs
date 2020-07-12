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

    #[error("Failed to read link catalog {0}")]
    LinkCatalog(PathBuf),

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

mod config;
pub mod indexer;
mod file;
pub mod filter;
pub mod link;
pub mod resolve;
mod page;
mod profile;

pub use config::*;
pub use page::Page;
pub use file::{FileType, FileInfo, FileOptions};
pub use profile::{ProfileName, ProfileSettings};
pub use indexer::IndexQuery;
