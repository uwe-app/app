use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use url::Url;

use crate::{
    script::ScriptAsset, style::StyleAsset, utils::href::UrlPath,
    TemplateEngine, ASSETS, PLUGINS,
};

use super::{dependency::DependencyMap, features::FeatureMap};

// TODO: spdx license for Plugin and ExternalLibrary

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
    /// Format for a content type, eg: book or slideshow.
    #[serde(rename = "format")]
    Format,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum PluginKind {
    One(PluginType),
    Many(Vec<PluginType>),
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

    /// List of third-party libraries the plugin depends on.
    pub library: Option<Vec<ExternalLibrary>>,

    /// List of synthetic assets to include in the project.
    pub assets: Option<Vec<UrlPath>>,

    // TODO: support arbitrary files which may be pages!
    /// List of stylesheets to add to pages.
    pub styles: Option<Vec<StyleAsset>>,

    /// List of scripts to add to pages.
    pub scripts: Option<Vec<ScriptAsset>>,

    /// Plugin dependencies.
    pub dependencies: Option<DependencyMap>,

    /// Collection of features for this plugggin.
    #[serde(flatten)]
    pub features: Option<FeatureMap>,

    /// Collections of partials and layouts
    #[serde(flatten)]
    pub templates: Option<HashMap<TemplateEngine, PluginTemplates>>,

    /// Base path this plugin was loaded from,
    /// used to resolve assets during collation.
    #[serde(skip)]
    pub base: PathBuf,

    /// A checksum digest when extracted from a registry archive.
    #[serde(skip)]
    pub checksum: Option<String>,

    /// A source URL the plugin was loaded from.
    #[serde(skip)]
    pub source: Option<Url>,
}

impl fmt::Display for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", &self.name, self.version.to_string())
    }
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
            features: None,
            templates: None,
            library: None,
            base: PathBuf::from(String::new()),
            checksum: None,
            source: None,
        }
    }
}

impl Plugin {
    /// Generate a qualified name relative to the plugin name.
    pub fn qualified(&self, val: &str) -> String {
        format!("{}::{}", &self.name, val)
    }

    /// Get the path for the plugin assets.
    pub fn assets(&self) -> PathBuf {
        PathBuf::from(ASSETS).join(PLUGINS).join(&self.name)
    }

    /// Resolve a URL path relative to this plugin.
    pub fn to_path_buf(&self, path: &UrlPath) -> PathBuf {
        self.base
            .join(utils::url::to_path_separator(path.trim_start_matches("/")))
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalLibrary {
    /// Library version.
    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,

    /// Library website.
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub website: Option<Url>,

    /// Library repository.
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub repository: Option<Url>,
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
        base.join(utils::url::to_path_separator(
            self.file.trim_start_matches("/"),
        ))
    }
}
