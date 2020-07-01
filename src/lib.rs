use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Ignore(#[from] ignore::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    Git(#[from] git2::Error),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Compiler(#[from] compiler::Error),
    #[error(transparent)]
    Locale(#[from] locale::Error),
    #[error(transparent)]
    Workspace(#[from] workspace::Error),
    #[error(transparent)]
    GitLib(#[from] git::Error),
    #[error(transparent)]
    Preference(#[from] preference::Error),
    #[error(transparent)]
    Cache(#[from] cache::Error),
    #[error(transparent)]
    Updater(#[from] updater::Error),
    #[error(transparent)]
    Report(#[from] report::Error),
    #[error(transparent)]
    Publish(#[from] publisher::Error),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

mod command;

pub use crate::command::blueprint;
pub use crate::command::build;
pub use crate::command::docs;
pub use crate::command::fetch;
pub use crate::command::run;
pub use crate::command::publish;
pub use crate::command::site;
pub use crate::command::upgrade;

pub use config::{BuildArguments, Config};
