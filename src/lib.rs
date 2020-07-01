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
    Compiler(#[from] compiler::Error),
    #[error(transparent)]
    Locale(#[from] locale::Error),

    #[error(transparent)]
    GitLib(#[from] git::error::GitError),
    #[error(transparent)]
    Preference(#[from] preference::Error),
    #[error(transparent)]
    Cache(#[from] cache::CacheError),
    #[error(transparent)]
    Updater(#[from] updater::UpdaterError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Report(#[from] report::ReportError),
    #[error(transparent)]
    Aws(#[from] publisher::AwsError),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

mod command;
mod workspace;

pub use crate::command::blueprint;
pub use crate::command::build;
pub use crate::command::docs;
pub use crate::command::fetch;
pub use crate::command::run;
pub use crate::command::publish;
pub use crate::command::site;
pub use crate::command::upgrade;

pub use config::{BuildArguments, Config};
