use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Build tag may not be an absolute path {0}")]
    BuildTagAbsolute(String),

    #[error("Live reload is not available for release builds")]
    LiveReloadRelease,

    #[error("Missing layout file {0}")]
    NoLayout(PathBuf),

    #[error("Profiles may not define a build tag, please remove it")]
    NoProfileBuildTag,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Compiler(#[from] compiler::Error),
    #[error(transparent)]
    Locale(#[from] locale::Error),
    #[error(transparent)]
    DataSource(#[from] datasource::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod compile;
mod finder;
mod merge;
pub mod project;

pub use compile::compile_project;
pub use compile::compile;
pub use compile::build;
pub use finder::find;
