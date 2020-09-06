use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

pub mod collation;
pub mod collator;
pub mod loader;
pub mod locale;
pub mod manifest;
pub mod resource;

pub use collation::*;
pub use collator::*;
pub use locale::*;
pub use resource::*;

use config::LocaleName;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No layout file found in source {0} with value {1}")]
    NoLayout(PathBuf, PathBuf),

    #[error("Collision detected on {0} ({1} <-> {2})")]
    LinkCollision(String, PathBuf, PathBuf),

    #[error("File {0} for page data with key {1} does not exist")]
    NoPageFile(PathBuf, String),

    #[error("Front matter error in {0} ({1})")]
    FrontMatterParse(PathBuf, toml::de::Error),

    #[error(
        "Duplicate permalink for path '{0}', ensure permalinks are unique"
    )]
    DuplicatePermalink(String),

    #[error("Series '{0}' references missing page {1}")]
    NoSeriesPage(String, PathBuf),

    #[error("Series '{0}' has duplicate page {1}")]
    DuplicateSeriesPage(String, PathBuf),

    #[error("Query may not combine 'each' with 'page'")]
    QueryConflict,

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    Poison(#[from] std::sync::PoisonError<CollateInfo>),

    #[error(transparent)]
    PoisonTranslations(
        #[from] std::sync::PoisonError<HashMap<LocaleName, CollateInfo>>,
    ),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    FrontMatter(#[from] frontmatter::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
}

type Result<T> = std::result::Result<T, Error>;
