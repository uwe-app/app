use thiserror::Error;
use config::Config;

mod compile;
mod finder;
mod merge;
pub mod project;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

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
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Workspace {
    pub config: Config,
}

impl Workspace {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

pub use finder::find;
pub use compile::compile;
pub use compile::compile_from;
pub use compile::compile_project;
