use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No version available, perform an installation first")]
    NotInstalled,

    #[error("Version {0} could not be found")]
    VersionNotFound(String),

    #[error("Version {0} is not installed ({1})")]
    VersionNotInstalled(String, PathBuf),

    #[error("Version {0} is the current version, use another version before removal")]
    NoRemoveCurrent(String),

    #[error("Version {0} is not a valid semver")]
    InvalidVersion(String),

    #[error("Release version {0} already exists")]
    ReleaseVersionExists(String),

    #[error("The build artifact {0} is not a file")]
    NoBuildArtifact(PathBuf),

    #[error("Download failed; status: {0}, url: {1}")]
    DownloadFail(String, String),

    #[error("Digests do not match for {0} ({1} != {2})")]
    DigestMismatch(String, String, String),

    #[error("No releases found, check an `update` semver range matches released versions")]
    NoReleasesFound,

    #[error("Range filters cannot be used on the first installation")]
    RangeFilterNotAllowedOnFirstRun,

    #[error("Unable to parse version in {0} ({1})")]
    VersionFileRead(PathBuf, String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    SemverParse(#[from] semver::ReqParseError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Crossterm(#[from] crossterm::ErrorKind),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Publisher(#[from] publisher::Error),

    #[error(transparent)]
    Scm(#[from] scm::Error),

    #[error(transparent)]
    Plugin(#[from] plugin::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

mod binary;
mod checksum;
mod download;
mod env;
mod install;
mod list;
mod publish;
mod releases;
mod remove;
mod uninstall;
mod update;
mod verify;
mod version;

pub use install::{install, select};
pub use list::list;
pub use publish::publish;
pub use releases::mount;
pub use remove::{prune, remove};
pub use uninstall::uninstall;
pub use update::{update, update_self};
pub use version::{default_version, find_local_version};
