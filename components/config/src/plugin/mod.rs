use std::collections::hash_map;
use std::collections::HashMap;
use std::path::PathBuf;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use url::Url;

use crate::{script::ScriptAsset, style::StyleAsset, utils::href::UrlPath};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyMap {
    #[serde(flatten)]
    pub items: HashMap<String, Dependency>,
}

impl DependencyMap {
    pub fn into_iter(self) -> hash_map::IntoIter<String, Dependency> {
        self.items.into_iter()
    }

    pub fn to_vec(&self) -> Vec<(&String, &Dependency)> {
        let out: Vec<(&String, &Dependency)> = Vec::new();
        self.items.iter().fold(out, |mut acc, (name, dep)| {
            acc.push((name, dep));
            acc
        })
    }
}

/// Hint as to the type of plugin.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PluginType {
    /// Assets to be bundled with the website files.
    #[serde(rename = "asset")]
    Asset,
    /// Icon assets to be bundled with the website files.
    #[serde(rename = "icon")]
    Icon,
    /// Script(s) to be included with pages.
    #[serde(rename = "script")]
    Script,
    /// Style(s) to be included with pages.
    #[serde(rename = "style")]
    Style,
    /// Register one or more partial files.
    #[serde(rename = "partial")]
    Partial,
    /// Font pack; assets and style files.
    #[serde(rename = "font")]
    Font,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Partial {
    pub file: PathBuf,
    pub schema: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum PluginPartial {
    One(Partial),
    Many(Vec<PluginPartial>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum PluginKind {
    One(PluginType),
    Many(Vec<PluginType>),
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    /// Required version for the dependency.
    #[serde_as(as = "DisplayFromStr")]
    pub version: VersionReq,

    /// Path for a local file system plugin.
    pub path: Option<PathBuf>,

    /// Resolved plugin for this dependency.
    #[serde(skip)]
    pub plugin: Option<Plugin>,
}

/// Represents a single plugin definition.
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Plugin {
    /// Name of the plugin.
    pub name: String,

    /// Description of the plugin function.
    pub description: String,

    /// Plugin version.
    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,

    /// Base path this plugin was loaded from,
    /// used to resolve assets during collation.
    #[serde(skip)]
    pub base: PathBuf,

    /// Type of the plugin.
    #[serde(rename = "type")]
    pub kind: Option<PluginKind>,

    // List of remote orgins used by this plugin.
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub origins: Option<Vec<Url>>,

    /// Partial definition.
    pub partial: Option<PluginPartial>,

    /// List of synthetic assets to include in the project.
    pub assets: Option<Vec<UrlPath>>,

    /// List of stylesheets to add to pages.
    pub styles: Option<Vec<StyleAsset>>,

    /// List of scripts to add to pages.
    pub scripts: Option<Vec<ScriptAsset>>,

    /// Plugin dependencies.
    pub dependencies: Option<DependencyMap>,
}

impl Default for Plugin {
    fn default() -> Self {
        let version: Version = "0.0.0".parse().unwrap();
        Self {
            name: String::new(),
            description: String::new(),
            version,
            base: PathBuf::from(String::new()),
            kind: None,
            origins: None,
            partial: None,
            assets: None,
            styles: None,
            scripts: None,
            dependencies: None,
        }
    }
}
