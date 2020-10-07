use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {

    #[error("Release version {0} already exists")]
    ReleaseVersionExists(String),

    #[error("The build artifact {0} is not a file")]
    NoBuildArtifact(PathBuf),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod checksum;
mod releases;
mod publisher;

pub use publisher::publish;
