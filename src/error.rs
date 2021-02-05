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

    #[error("Not a file {0}")]
    NotFile(PathBuf),

    #[error("Path {0} is absolute but a relative path is required")]
    NotRelative(PathBuf),

    #[error("Refusing to overwrite {0}, please move it away to initialize integration tests")]
    NoOverwriteTestSpec(PathBuf),

    #[error("Target {0} exists, please move it away")]
    TargetExists(PathBuf),

    #[error("Folder {0} does not contain a settings file {1}")]
    NoSiteSettings(PathBuf, String),

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

    #[error("Plugin {0}@{1} for new project should be of type 'blueprint' but got '{2}'")]
    BlueprintPluginInvalidType(String, String, String),

    #[error("Plugin {0}@{1} is already installed, use --force to overwrite")]
    PluginAlreadyInstalled(String, String),

    #[error("To add a plugin requires a name, path, archive or URL.")]
    PluginAddNoTarget,

    #[error("Plugin targets cannot be mixed; use a plugin name or an option (--path, --archive, --git)")]
    PluginAddMultipleTargets,

    #[error(
        "New projects must have one source; use a plugin name, --path or --git"
    )]
    NewProjectMultipleSource,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

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
    ParseRegion(#[from] rusoto_core::region::ParseRegionError),

    #[error(transparent)]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),

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
    WebHost(#[from] web_host::Error),

    #[error(transparent)]
    Shim(#[from] crate::shim::Error),
}
