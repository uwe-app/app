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

    #[error("Plugin path {0} does not exist")]
    BadPluginPath(PathBuf),

    #[error("Plugin file {0} is not a file")]
    BadPluginFile(PathBuf),

    #[error("Plugin name may not be empty")]
    LintPluginNameEmpty,

    #[error("Plugin names must contain at least one namespace (::)")]
    LintPluginNameSpace,

    #[error("Plugin names contains invalid namespace {0} ([a-zA-Z0-9_-] only)")]
    LintPluginNameInvalidNameSpace(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Semver(#[from] config::semver::SemVerError),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod archive;
mod resolver;
mod linter;
mod walk;

pub use linter::lint;
pub use resolver::{solve, read};
