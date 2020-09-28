use std::io;
use std::path::{Path, PathBuf};

use log::info;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

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

    #[error("Dependency stack depth has exceeded the maximum {0}")]
    DependencyStackTooLarge(usize),

    #[error("Plugin path {0} does not exist")]
    BadPluginPath(PathBuf),

    #[error("Plugin file {0} is not a file")]
    BadPluginFile(PathBuf),

    #[error("Plugin name may not be empty")]
    LintPluginNameEmpty,

    #[error("Plugin description may not be empty")]
    LintPluginDescriptionEmpty,

    #[error("Plugin names must contain at least one namespace (::)")]
    LintPluginNameSpace,

    #[error("Plugin paths may not be absolute {0}")]
    LintNoAbsolutePath(String),

    #[error("Plugin asset {0} for path {1} is not a file")]
    LintNoPluginFile(PathBuf, String),

    #[error(
        "Plugin names contains invalid namespace {0} ([a-zA-Z0-9_-] only)"
    )]
    LintPluginNameInvalidNameSpace(String),

    #[error("Plugin {0} has invalid feature reference {1}")]
    LintFeatureMissing(String, String),

    #[error("Feature references dependency {0}@{1} which is not optional")]
    LintFeatureDependencyNotOptional(String, String),

    #[error("The archive package {0} already exists, please move it away")]
    PackageExists(PathBuf),

    #[error("The archive source path {0} is not a file")]
    PackageSourceNotFile(PathBuf),

    #[error("The archive target path {0} is not a directory")]
    PackageTargetNotDirectory(PathBuf),

    #[error("Package digests do not match")]
    DigestMismatch(PathBuf),

    #[error("Invalid archive {0} no {1} found")]
    InvalidArchiveNoPluginFile(PathBuf, String),

    #[error("Registry {0} is not a directory")]
    RegistryNotDirectory(PathBuf),

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

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Hex(#[from] hex::FromHexError),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Semver(#[from] config::semver::SemVerError),

    #[error(transparent)]
    PathPersist(#[from] tempfile::PathPersistError),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Xz(#[from] xz2::stream::Error),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Cache(#[from] cache::Error),

    #[error(transparent)]
    Preference(#[from] preference::Error),

    #[error(transparent)]
    Publisher(#[from] publisher::Error),

    #[error(transparent)]
    Git(#[from] git::Error),
}

mod archive;
mod installer;
mod linter;
mod packager;
mod registry;
mod resolver;
mod uploader;
mod walk;

type Result<T> = std::result::Result<T, Error>;
pub type Registry<'r> = Box<dyn registry::RegistryAccess + Send + Sync + 'r>;

pub use linter::lint;
pub use packager::pack;
pub use resolver::{read, resolve};
pub use uploader::publish;
