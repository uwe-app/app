use sha3::{Digest, Sha3_256};
use std::io::Write;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Project path {0} is relative, must be an absolute path")]
    NoRelativeProject(PathBuf),

    #[error("Project {0} already exists")]
    Exists(PathBuf),

    #[error("Project {0} does not exist")]
    NotExists(PathBuf),

    #[error("Folder {0} does not contain a settings file {1}")]
    NoSiteSettings(PathBuf, String),

    #[error("Plugin {0}@{1} for new project should be of type 'blueprint' but got '{2}'")]
    BlueprintPluginInvalidType(String, String, String),

    #[error("Language {0} does not exist in the locales {1}")]
    LanguageMissingFromLocales(String, String),

    #[error("Target {0} exists, please move it away")]
    TargetExists(PathBuf),

    #[error(
        "New projects must have one source; use a plugin name, --path or --git"
    )]
    NewProjectMultipleSource,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Plugin(#[from] plugin::Error),

    #[error(transparent)]
    Preference(#[from] preference::Error),

    #[error(transparent)]
    Scm(#[from] scm::Error),

    #[error(transparent)]
    Utils(#[from] utils::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod bridge;
mod create;
mod manage;

pub use bridge::ConnectionBridge;
pub use create::{create, ProjectOptions};
pub use manage::{
    add, find, list, load, remove, import, ProjectList, ProjectManifestEntry,
};

/// Compute the SHA3-256 checksum of a project path.
pub(crate) fn digest<P: AsRef<Path>>(target: P) -> Result<Vec<u8>> {
    let mut hasher = Sha3_256::new();
    hasher.write(target.as_ref().to_string_lossy().as_bytes())?;
    Ok(hasher.finalize().as_slice().to_owned())
}

/// Compute the SHA3-256 checksum of a project path as a hex string.
pub fn checksum<P: AsRef<Path>>(target: P) -> Result<String> {
    let checksum = digest(target)?;
    let s = checksum
        .iter()
        .map(|b| format!("{:x}", b))
        .collect::<Vec<_>>()
        .join("");
    Ok(s)
}
