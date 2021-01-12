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

    //#[error("No virtual hosts for live reload")]
    //NoLiveHosts,
    #[error("Live reload does not support the ephemeral port")]
    NoLiveEphemeralPort,

    #[error("No publish configuration")]
    NoPublishConfiguration,

    #[error("Unknown publish environment {0}")]
    UnknownPublishEnvironment(String),

    #[error("Plugin publishing is not available yet")]
    NoPluginPublishPermission,

    #[error("Could not determine a target name")]
    NoTargetName,

    #[error("Repository {0} is not a valid URL")]
    InvalidRepositoryUrl(String),

    #[error("Not a repository {0}")]
    NotRepository(PathBuf),

    #[error("Server could not find {0} or {1} in {2}")]
    NoServerFile(String, String, PathBuf),

    #[error("Local dependency {0} is not allowed")]
    LocalDependencyNotAllowed(PathBuf),

    #[error("Plugin {0}@{1} for project blueprint should be of type 'site' but got '{2}'")]
    BlueprintPluginNotSiteType(String, String, String),

    #[error("Plugin {0}@{1} is already installed, use --force to overwrite")]
    PluginAlreadyInstalled(String, String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    ReqParse(#[from] semver::ReqParseError),

    #[error(transparent)]
    LanguageIdentifier(#[from] unic_langid::LanguageIdentifierError),

    #[error(transparent)]
    Crossterm(#[from] crossterm::ErrorKind),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Compiler(#[from] compiler::Error),

    #[error(transparent)]
    Locale(#[from] locale::Error),

    #[error(transparent)]
    Workspace(#[from] workspace::Error),

    #[error(transparent)]
    Scm(#[from] scm::Error),

    #[error(transparent)]
    Preference(#[from] preference::Error),

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

    #[error(transparent)]
    Shim(#[from] crate::shim::Error),
}
