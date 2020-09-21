use std::collections::hash_map;
use std::collections::HashMap;
use std::path::PathBuf;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use url::Url;

use crate::{
    Result,
    TemplateEngine,
    script::ScriptAsset,
    style::StyleAsset,
    utils::href::UrlPath,
};

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

            if let Some(ref plugin) = dep.plugin {
                if let Some(ref dependencies) = plugin.dependencies {
                    let mut deps = dependencies.to_vec(); 
                    acc.append(&mut deps);
                }
            }

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

impl Dependency {

    /// Cache glob patterns used to apply plugins to 
    /// files.
    pub fn prepare(&mut self) -> Result<()> {
        println!("Dependency preparing...");
        Ok(()) 
    }
}

/// Represents a plugin definition.
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

    /// List of keywords.
    pub keywords: Option<Vec<String>>,

    /// Type of the plugin.
    #[serde(rename = "type")]
    pub kind: Option<PluginKind>,

    /// List of remote orgins used by this plugin.
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub origins: Option<Vec<Url>>,

    /// List of synthetic assets to include in the project.
    pub assets: Option<Vec<UrlPath>>,

    /// List of stylesheets to add to pages.
    pub styles: Option<Vec<StyleAsset>>,

    /// List of scripts to add to pages.
    pub scripts: Option<Vec<ScriptAsset>>,

    /// Plugin dependencies.
    pub dependencies: Option<DependencyMap>,

    /// Collections of partials and layouts.
    #[serde(flatten)]
    pub templates: Option<HashMap<TemplateEngine, PluginTemplates>>,

    /// Base path this plugin was loaded from,
    /// used to resolve assets during collation.
    #[serde(skip)]
    pub base: PathBuf,
}

impl Default for Plugin {
    fn default() -> Self {
        let version: Version = "0.0.0".parse().unwrap();
        Self {
            name: String::new(),
            description: String::new(),
            version,
            keywords: None,
            kind: None,
            origins: None,
            assets: None,
            styles: None,
            scripts: None,
            dependencies: None,
            templates: None,
            base: PathBuf::from(String::new()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginTemplates {
    /// Partial definitions.
    pub partials: Option<HashMap<String, TemplateAsset>>,

    /// Layout definitions.
    pub layouts: Option<HashMap<String, TemplateAsset>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateAsset {
    pub file: UrlPath,
    pub schema: Option<UrlPath>,
}

impl TemplateAsset {
    pub fn to_path_buf(&self, base: &PathBuf) -> PathBuf {
        base.join(
            utils::url::to_path_separator(
                self.file.trim_start_matches("/")))
    }
}
