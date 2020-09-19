use std::collections::HashMap;
use std::path::PathBuf;

use semver::{Version, VersionReq};

use serde::{Deserialize, Serialize};

pub type DependencyMap = HashMap<String, Dependency>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {

    /// Required version for the dependency.
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub version: VersionReq,

    /// Path for a local file system plugin.
    pub path: Option<PathBuf>,

    /// Resolved plugin for this dependency.
    #[serde(skip)]
    pub plugin: Option<Plugin>,
}

/// Represents a single plugin definition.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    /// Name of the plugin.
    pub name: String,

    /// Description of the plugin function.
    pub description: String,

    /// Plugin version.
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub version: Version,

    /// List of synethetic assets to include in the project.
    pub assets: Vec<PathBuf>,

    /// List of stylesheets to add to pages.
    pub stylesheets: Vec<PathBuf>,

    /// List of scripts to add to pages.
    pub scripts: Vec<PathBuf>,

    /// Plugin dependencies.
    pub dependencies: Option<DependencyMap>,
}
