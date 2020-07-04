use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BundleError {
    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Unknown path type")]
    UnknownPathType,

    #[error("Failed to determine file name")]
    NoFileName,

    #[error("Failed to get file meta data")]
    NoFileMetaData,

    #[error("Bundle {0} already exists, use --force to overwrite")]
    BundleExists(PathBuf),

    #[error("Could not execute 'go version', install from https://golang.org/dl/")]
    NoToolChain,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Git(#[from] git::Error),

    #[error(transparent)]
    Cache(#[from] cache::Error),

    #[error(transparent)]
    Preference(#[from] preference::Error),
}

mod bundler;
mod command;

pub use command::bundle;
pub use command::BundleOptions;
