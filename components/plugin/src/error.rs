use std::path::PathBuf;

use bracket::error::SyntaxError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LintError {
    #[error("Plugin name may not be empty")]
    LintPluginNameEmpty,

    #[error("Plugin description may not be empty")]
    LintPluginDescriptionEmpty,

    #[error("Plugin names must contain at least one namespace (::)")]
    LintPluginNameSpace,

    #[error("Plugin paths may not be absolute {0}")]
    LintNoAbsolutePath(String),

    #[error("License is not a valid SPDX identifier {0}")]
    LintLicenseNotSpdx(String),

    #[error("Plugin asset {0} for path {1} is not a file")]
    LintNoPluginFile(PathBuf, String),

    #[error("Plugin names contains invalid namespace {0} ([a-zA-Z0-9-] only)")]
    LintPluginNameInvalidNameSpace(String),

    #[error("Plugin {0} has invalid feature reference {1}")]
    LintFeatureMissing(String, String),

    #[error("Feature references dependency {0}@{1} which is not optional")]
    LintFeatureDependencyNotOptional(String, String),

    #[error("Features are not supported for the `site` plugin type")]
    LintFeaturesSiteType,

    #[error("Plugin file {0} is outside {1}, possible symbolic link error")]
    LintSymbolicLink(PathBuf, PathBuf),

    #[error("{0}")]
    Syntax(String),

    #[error(transparent)]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
}

impl From<SyntaxError> for LintError {
    fn from(e: SyntaxError) -> Self {
        LintError::Syntax(format!("{:?}", e))
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Plugin path is not a directory {0}")]
    PluginPathNotDirectory(PathBuf),

    #[error("No package or plugin could be found for dependency {0}")]
    DependencyNotFound(String),

    #[error("Incompatible dependency versions; {0} does not satisfy existing version {1}")]
    IncompatibleDependency(String, String),

    #[error("Plugin key {0} does not match plugin name {1}")]
    PluginNameMismatch(String, String),

    #[error("Plugin {0}@{1} does not satsify requirement {2}")]
    PluginVersionMismatch(String, String, String),

    #[error("Cyclic dependency {0}")]
    CyclicDependency(String),

    #[error("Cyclic plugin {0}")]
    CyclicPlugin(PathBuf),

    #[error("Could not parse scm repository URL ({0})")]
    GitUrl(url::ParseError),

    #[error("Dependency stack depth has exceeded the maximum {0}")]
    DependencyStackTooLarge(usize),

    #[error("Plugin stack depth has exceeded the maximum {0}")]
    PluginStackTooLarge(usize),

    #[error("Plugin path {0} does not exist")]
    BadPluginPath(PathBuf),

    #[error("Plugin file {0} is not a file")]
    BadPluginFile(PathBuf),

    #[error("The archive package {0} already exists, please move it away")]
    PackageExists(PathBuf),

    #[error("The archive source path {0} is not a file")]
    PackageSourceNotFile(PathBuf),

    #[error("The archive target path {0} is not a directory")]
    PackageTargetNotDirectory(PathBuf),

    #[error("Package digests do not match {0}")]
    DigestMismatch(PathBuf),

    #[error("Invalid archive {0} no {1} found")]
    InvalidArchiveNoPluginFile(PathBuf, String),

    #[error("Registry {0} is not a directory")]
    RegistryNotDirectory(PathBuf),

    #[error("Download from registry failed; status: {0}, url: {1}")]
    RegistryDownloadFail(String, String),

    #[error(
        "Plugin {0} already exists in the registry, use a different version"
    )]
    RegistryPluginVersionExists(String),

    #[error("Plugin repository {0} must be in a clean state")]
    RegistryNotClean(String),

    #[error("Package {0} does not exist in the registry")]
    RegistryPackageNotFound(String),

    #[error("Package {0} exists but no version found matching {1}")]
    RegistryPackageVersionNotFound(String, String),

    #[error("Attempt to install local scope {0} without plugins")]
    PluginWithNoParentScope(String),

    #[error("The scoped plugin {0} could not be found in the plugins")]
    PluginScopeNotFound(String),

    #[error(
        "The parent plugin {0} could not be found for the scoped plugin {1}"
    )]
    PluginParentNotFound(String, String),

    #[error("The dependency {0} uses the local scope {1} but no plugins are available")]
    PluginScopeRequiresParent(String, String),

    #[error("Plugin directory {0} does not exist, remove the lock file and try again")]
    NoPluginInstallDir(PathBuf),

    #[error("Plugin {0} could not be found; update the plugin registry with `uwe task update-registry` and try again")]
    PluginNotFound(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    ReqParse(#[from] semver::ReqParseError),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Hex(#[from] hex::FromHexError),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    UrlEncoding(#[from] urlencoding::FromUrlEncodingError),

    #[error(transparent)]
    Semver(#[from] config::semver::SemVerError),

    #[error(transparent)]
    PathPersist(#[from] tempfile::PathPersistError),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    Xz(#[from] xz2::stream::Error),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    //#[error(transparent)]
    //Regex(#[from] regex::Error),
    #[error(transparent)]
    Lint(#[from] self::LintError),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Publisher(#[from] publisher::Error),

    #[error(transparent)]
    Scm(#[from] scm::Error),
}
