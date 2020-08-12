use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("The path filter {0} does not exist")]
    NoFilter(PathBuf),

    #[error("Build tag may not be an absolute path {0}")]
    ProfileNameAbsolute(String),

    #[error("Permalink {0} collides with an existing redirect")]
    RedirectPermalinkCollision(String),

    #[error("Live reload is not available for release builds")]
    LiveReloadRelease,

    #[error("Missing layout file {0}")]
    NoLayout(PathBuf),

    #[error("Profiles may not define a build profile, please remove it")]
    NoProfileInProfile,

    #[error(transparent)]
    Io(#[from] std::io::Error),

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

mod compile;
mod finder;
mod merge;
pub mod project;

pub use compile::compile;
pub use compile::compile_project;
//pub use compile::build;
pub use finder::find;
