use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No version available, perform an installation first")]
    NotInstalled,

    #[error("Release version {0} already exists")]
    ReleaseVersionExists(String),

    #[error("The build artifact {0} is not a file")]
    NoBuildArtifact(PathBuf),

    #[error("Download failed; status: {0}, url: {1}")]
    DownloadFail(String, String),

    #[error("Digests do not match for {0} ({1} != {2})")]
    DigestMismatch(String, String, String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Publisher(#[from] publisher::Error),

    #[error(transparent)]
    Cache(#[from] cache::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod binary;
mod checksum;
mod download;
mod env;
mod install;
mod list;
mod publish;
mod runtime;
mod releases;
mod uninstall;
mod upgrade;
mod version;

pub use install::{install, latest};
pub use list::list;
pub use publish::publish;
pub use runtime::update;
pub use uninstall::uninstall;
pub use upgrade::upgrade;
