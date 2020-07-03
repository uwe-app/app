use thiserror::Error;

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
    #[error(transparent)]
    DataSource(#[from] datasource::Error),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

type Result<T> = std::result::Result<T, Error>;

pub use compile::compile;
pub use compile::compile_from;
pub use compile::compile_project;
pub use finder::find;
