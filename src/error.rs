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

    #[error("Target {0} exists, please move it away")]
    TargetExists(PathBuf),

    #[error("Folder {0} does not contain a settings file {1}")]
    NoSiteSettings(PathBuf, String),

    #[error("Unable to determine a source for the new project, please check the <source> option")]
    NoInitSource,

    #[error("Language {0} does not exist in the locales {1}")]
    LanguageMissingFromLocales(String, String),

    #[error("No virtual hosts for live reload")]
    NoLiveHosts,

    #[error("Live reload does not support the ephemeral port")]
    NoLiveEphemeralPort,

    #[error("No publish configuration")]
    NoPublishConfiguration,

    #[error("Unknown publish environment {0}")]
    UnknownPublishEnvironment(String),

    #[error("Plugin publishing is not available yet")]
    NoPluginPublishPermission,

    //#[error("No socket address for {0}")]
    //NoSocketAddress(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

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
    #[error(transparent)]
    Server(#[from] server::Error),
    #[error(transparent)]
    Plugin(#[from] plugin::Error),
    #[error(transparent)]
    Utils(#[from] utils::Error),
    #[error(transparent)]
    Release(#[from] release::Error),
}
