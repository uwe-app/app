use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Book(#[from] book::Error),
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
    #[error(transparent)]
    Site(#[from] site::Error),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod command;

pub use config::{BuildArguments, Config};
