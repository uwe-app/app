use std::io;
use std::collections::HashSet;
use std::path::PathBuf;

use thiserror::Error;

use config::lock_file::LockFileEntry;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Plugin key {0} does not match plugin name {1}")]
    PluginNameMismatch(String, String),

    #[error("Plugin {0}@{1} does not satsify requirement {2}")]
    PluginVersionMismatch(String, String, String),

    #[error("Plugin cyclic dependency: {0}")]
    PluginCyclicDependency(String),

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

    #[error(
        "Plugin names contains invalid namespace {0} ([a-zA-Z0-9_-] only)"
    )]
    LintPluginNameInvalidNameSpace(String),

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

pub use archive::{reader::PackageReader, writer::PackageWriter};
pub use linter::lint;
pub use packager::pack;
pub use resolver::{read, solve};
pub use uploader::publish;

pub type Registry<'r> = Box<dyn registry::RegistryAccess + Send + Sync + 'r>;

pub fn new_registry<'r>() -> Result<Registry<'r>> {
    let reg = cache::get_registry_dir()?;
    Ok(Box::new(registry::RegistryFileAccess::new(
        reg.clone(),
        reg.clone(),
    )?))
}

pub async fn install(
    registry: &Registry<'_>,
    difference: HashSet<&LockFileEntry>) -> Result<()> {

    for entry in difference {
        println!("Install from lock file entry {}", &entry.name);
        println!("Entry {:#?}", &entry)
    }

    Ok(())
}
