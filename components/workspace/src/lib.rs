use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Path {0} is outside the site source")]
    OutsideSourceTree(PathBuf),

    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Global icon file {0} does not exist or is not a file ({1})")]
    NoMainIcon(String, PathBuf),

    #[error("Global script file {0} does not exist or is not a file ({1})")]
    NoMainScript(String, PathBuf),

    #[error("Global style file {0} does not exist or is not a file ({1})")]
    NoMainStyle(String, PathBuf),

    #[error("App manifest {0} does not exist or is not a file ({1})")]
    NoAppManifest(String, PathBuf),

    #[error("The path filter {0} does not exist")]
    NoFilter(PathBuf),

    #[error("Project workspaces may not be nested")]
    NoNestedWorkspace(PathBuf),

    #[error("Profile {0} does not exist")]
    NoProfile(String),

    #[error("Build source may not be an absolute path {0}")]
    SourceAbsolute(PathBuf),

    #[error("Build target may not be an absolute path {0}")]
    TargetAbsolute(PathBuf),

    #[error("Build profile may not be an absolute path {0}")]
    ProfileNameAbsolute(String),

    #[error("Redirect {0} collides with an existing redirect")]
    RedirectCollision(String),

    #[error("Syntax highlighting path {0} is not a directory")]
    NoSyntaxDirectory(PathBuf),

    #[error("Syntax highlighting requires the `{0}` plugin, add it to the project dependencies")]
    NoSyntaxPlugin(String),

    #[error("Plugin {0} is missing asset file {1}")]
    NoPluginAsset(String, PathBuf),

    #[error("Plugin {0} defines hooks but access is denied {1} (needs `enable = [\"hooks\"]`)")]
    NoHooksGrant(String, String),

    #[error("Plugin {0} references an absolute path {1}")]
    PluginAbsolutePath(String, PathBuf),

    #[error("Expected menu file {0} for book path {1}")]
    NoBookMenu(PathBuf, PathBuf),

    //#[error("Profiles may not define a build profile, please remove it")]
    //NoProfileInProfile,
    #[error("Failed to get canonical path for project root {0}")]
    CanonicalProjectRoot(PathBuf),

    #[error(r#"Project "{0}" requires execution capability"#)]
    NoExecCapability(String),

    #[error("Duplicate host name {0}: {1} <-> {2}")]
    DuplicateHostName(String, PathBuf, PathBuf),

    #[error(transparent)]
    Box(#[from] Box<dyn std::error::Error + Sync + Send>),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Preference(#[from] preference::Error),

    #[error(transparent)]
    Compiler(#[from] compiler::Error),

    #[error(transparent)]
    Locale(#[from] locale::Error),

    #[error(transparent)]
    Collections(#[from] collections::Error),

    #[error(transparent)]
    Collator(#[from] collator::Error),

    #[error(transparent)]
    Syntax(#[from] syntax::Error),

    #[error(transparent)]
    Search(#[from] search::Error),

    #[error(transparent)]
    Plugin(#[from] plugin::Error),

    #[error(transparent)]
    Release(#[from] release::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

mod hook;
mod invalidator;
pub mod lock;
mod manifest;
mod options;
mod plugins;
mod project;
mod renderer;

pub use invalidator::Invalidator;
pub use project::*;
