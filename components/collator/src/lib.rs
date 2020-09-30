use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use locale::LocaleName;

#[derive(Error, Debug)]
pub enum Error {
    //#[error("No layout definition {0} found for page {1}")]
    //NoLayoutDefinition(String, PathBuf),
    #[error("No plugin located for feed templates using plugin name {0}")]
    NoFeedPlugin(String),

    #[error(
        "The feed plugin {0} has no templates for the template engine {1}"
    )]
    NoFeedPluginTemplateEngine(String, String),

    #[error("The feed plugin {0} has no template partials")]
    NoFeedPluginPartial(String),

    #[error("Unable to determine template path for feed type {0}")]
    NoFeedPartialPath(String),

    #[error("No feed template file {0}")]
    NoFeedTemplate(PathBuf),

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

    #[error("Query may not combine 'each' with 'page'")]
    QueryConflict,

    #[error("No page found for menu item reference {0}")]
    NoMenuItem(String),

    #[error("No page data found for menu item path {0}")]
    NoMenuItemPage(PathBuf),

    #[error("Menu file {0} contains a link {1} which does not exist ({2})")]
    NoMenuLink(PathBuf, String, PathBuf),

    #[error("Menu file {0} contains a link {1} which could not be resolved to a path")]
    NoMenuPagePath(PathBuf, String),

    #[error("Menu file {0} contains a link {1} which could not be resolved to a page ({2})")]
    NoMenuPage(PathBuf, String, PathBuf),

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
pub use synthetic::{create_file, create_page, feed};
