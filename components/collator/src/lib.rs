use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use locale::LocaleName;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No layout definition {0} found for page {1}")]
    NoLayoutDefinition(String, PathBuf),

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

    #[error("No page found for menu item reference {0}")]
    NoMenuItem(String),

    #[error("No page data found for menu item path {0}")]
    NoMenuItemPage(PathBuf),

    #[error("No feed template file {0}")]
    NoFeedTemplate(PathBuf),

    #[error("No book theme directory {0}")]
    NoBookThemeDirectory(PathBuf),

    #[error("No layout file {0} for book theme directory {1}")]
    NoBookThemeLayout(PathBuf, PathBuf),

    #[error(transparent)]
    Format(#[from] std::fmt::Error),

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
    Url(#[from] url::ParseError),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    FrontMatter(#[from] frontmatter::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod builder;
pub mod collation;
pub mod collator;
pub mod loader;
pub mod locale_utils;
pub mod menu;
pub mod resource;
mod synthetic;

pub use builder::to_href;
pub use collation::*;
pub use collator::*;
pub use locale_utils::*;
pub use resource::*;
pub use synthetic::{create_page, create_file, feed, search, book};
