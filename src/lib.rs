use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Panic(String),

    #[error("Unknown log level {0}")]
    UnknownLogLevel(String),

    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Target directory is required")]
    TargetRequired,

    #[error("Target {0} exists, please move it away")]
    TargetExists(PathBuf),

    #[error("Book creation requires a path")]
    BookCreatePath,

    #[error("Book creation requires a project not a workspace")]
    BookCreateWorkspace,

    #[error("Language {0} does not exist in the locales {1}")]
    LanguageMissingFromLocales(String, String),

    #[error("Could not determine default source path")]
    SourceEmpty,

    #[error("No publish configuration")]
    NoPublishConfiguration,

    #[error("Unknown publish environment {0}")]
    UnknownPublishEnvironment(String),

    //#[error("No socket address for {0}")]
    //NoSocketAddress(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Notify(#[from] notify::Error),

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

pub type Result<T> = std::result::Result<T, Error>;
pub type ErrorCallback = fn(Error);

pub mod command;

pub use config::{Config, ProfileSettings};
