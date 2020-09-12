use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Path {0} is outside the site source")]
    OutsideSourceTree(PathBuf),

    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("The path filter {0} does not exist")]
    NoFilter(PathBuf),

    #[error("Project workspaces may not be nested")]
    NoNestedWorkspace(PathBuf),

    #[error("Build tag may not be an absolute path {0}")]
    ProfileNameAbsolute(String),

    #[error("Permalink {0} collides with an existing redirect")]
    RedirectPermalinkCollision(String),

    #[error("Live reload is not available for release builds")]
    LiveReloadRelease,

    #[error("Invalidation action not handled")]
    InvalidationActionNotHandled,

    #[error("Missing layout file {0}")]
    NoLayout(PathBuf),

    #[error("Profiles may not define a build profile, please remove it")]
    NoProfileInProfile,

    #[error("Failed to get canonical path for project root {0}")]
    CanonicalProjectRoot(PathBuf),

    #[error("No page found for menu item reference {0}")]
    NoMenuItem(String),

    #[error("no page data found for menu item path {0}")]
    NoMenuItemPage(PathBuf),

    #[error(transparent)]
    Format(#[from] std::fmt::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Cache(#[from] cache::Error),
    #[error(transparent)]
    Preference(#[from] preference::Error),
    #[error(transparent)]
    Compiler(#[from] compiler::Error),
    #[error(transparent)]
    Book(#[from] book::Error),
    #[error(transparent)]
    Locale(#[from] locale::Error),
    #[error(transparent)]
    DataSource(#[from] datasource::Error),
    #[error(transparent)]
    Collator(#[from] collator::Error),
    #[error(transparent)]
    Syntax(#[from] syntax::Error),
    #[error(transparent)]
    Search(#[from] search::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod hook;
mod invalidator;
pub mod lock;
mod manifest;
mod menu;
mod options;
mod project;
mod renderer;

pub use invalidator::Invalidator;
pub use project::*;
