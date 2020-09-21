use std::path::PathBuf;
use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {

    #[error("Plugin key {0} does not match plugin name {1}")]
    PluginNameMismatch(String, String),

    #[error("Plugin {0}@{1} does not satsify requirement {2}")]
    PluginVersionMismatch(String, String, String),

    #[error("Plugin cyclic dependency: {0}")]
    PluginCyclicDependency(String),

    #[error("Plugin path {0} is not a directory")]
    BadPluginPath(PathBuf),

    #[error("Plugin file {0} is not a file")]
    BadPluginFile(PathBuf),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Semver(#[from] config::semver::SemVerError),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod resolver;

pub use resolver::solve;
