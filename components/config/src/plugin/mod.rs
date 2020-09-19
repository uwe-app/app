use std::collections::HashMap;
use std::path::PathBuf;

use semver::{Version, VersionReq};

use serde::{Deserialize, Serialize};

pub type DependencyMap = HashMap<String, Dependency>;

/// Hint as to the type of plugin.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PluginType {
    /// Assets to be bundled with the website files.
    ///
    /// May also combine scripts and stylesheets.
    #[serde(rename = "assets")]
    Assets,
    /// Single partial with a schema to define the partial parameters.
    #[serde(rename = "shortcode")]
    ShortCode,
    /// Register one or more partial files.
    #[serde(rename = "partial")]
    Partial,
}

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

    /// Type of the plugin.
    #[serde(rename = "type")]
    pub kind: Option<PluginType>,

    /// List of synthetic assets to include in the project.
    pub assets: Option<Vec<PathBuf>>,

    /// List of stylesheets to add to pages.
    pub styles: Option<Vec<PathBuf>>,

    /// List of scripts to add to pages.
    pub scripts: Option<Vec<PathBuf>>,

    /// Plugin dependencies.
    pub dependencies: Option<DependencyMap>,
}
