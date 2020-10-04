use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use jsonfeed::Author;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use url::Url;

use crate::{
    engine::TemplateEngine,
    hook::HookMap,
    href::UrlPath,
    script::ScriptAsset,
    license::LicenseGroup,
    style::StyleAsset, ASSETS, PLUGINS,
};

use super::{dependency::DependencyMap, features::FeatureMap};

/// Hint as to the type of plugin.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PluginType {
    /// Build tool hook.
    #[serde(rename = "hook")]
    Hook,
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

    /// Plugin license.
    pub license: Option<LicenseGroup>,

    /// Plugin author(s).
    pub authors: Option<Vec<Author>>,

    /// List of keywords.
    pub keywords: Option<Vec<String>>,

    /// Type of the plugin.
    #[serde(rename = "type")]
    pub kind: Option<PluginKind>,

    /// List of remote orgins used by this plugin.
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub origins: Option<Vec<Url>>,

    /// Collection of features for this plugin.
    pub features: Option<FeatureMap>,

    /// Plugin dependencies.
    pub dependencies: Option<DependencyMap>,

    /// List of third-party libraries the plugin depends on.
    pub library: Option<HashMap<String, ExternalLibrary>>,

    /// List of synthetic assets to include in the project.
    assets: HashSet<UrlPath>,

    // NOTE: we want to use HashSet for styles and scripts
    // NOTE: so there are no duplicates but ordering is important
    // NOTE: for these types so we just use a Vec for now.
    /// List of stylesheets to add to pages.
    styles: Vec<StyleAsset>,

    /// List of scripts to add to pages.
    scripts: Vec<ScriptAsset>,

    /// Collections of partials and layouts
    #[serde(flatten, serialize_with = "toml::ser::tables_last")]
    pub templates: HashMap<TemplateEngine, PluginTemplates>,

    /// List of hooks in this plugin.
    pub hooks: Option<HookMap>,

    /// Base path this plugin was loaded from,
    /// used to resolve assets during collation.
    #[serde(skip)]
    base: PathBuf,

    /// A checksum digest when extracted from a registry archive.
    #[serde(skip)]
    checksum: Option<String>,

    /// A source URL the plugin was loaded from.
    #[serde(skip)]
    source: Option<Url>,
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
            license: None,
            authors: None,
            keywords: None,
            kind: None,
            origins: None,
            assets: HashSet::new(),
            styles: Vec::new(),
            scripts: Vec::new(),
            hooks: None,
            dependencies: None,
            features: None,
            templates: HashMap::new(),
            library: None,
            base: PathBuf::from(String::new()),
            checksum: None,
            source: None,
        }
    }
}

impl Plugin {
    pub fn base(&self) -> &PathBuf {
        &self.base
    }

    pub fn set_base<P: AsRef<Path>>(&mut self, p: P) {
        self.base = p.as_ref().to_path_buf();
    }

    pub fn source(&self) -> &Option<Url> {
        &self.source
    }

    pub fn set_source(&mut self, u: Url) {
        self.source = Some(u);
    }

    pub fn checksum(&self) -> &Option<String> {
        &self.checksum
    }

    pub fn set_checksum<S: AsRef<str>>(&mut self, s: S) {
        self.checksum = Some(s.as_ref().to_string());
    }

    pub fn assets(&self) -> &HashSet<UrlPath> {
        &self.assets
    }

    pub fn set_assets(&mut self, assets: HashSet<UrlPath>) {
        self.assets = assets;
    }

    pub fn styles(&self) -> &Vec<StyleAsset> {
        &self.styles
    }

    pub fn styles_mut(&mut self) -> &mut Vec<StyleAsset> {
        &mut self.styles
    }

    pub fn set_styles(&mut self, styles: Vec<StyleAsset>) {
        self.styles = styles;
    }

    pub fn scripts(&self) -> &Vec<ScriptAsset> {
        &self.scripts
    }

    pub fn scripts_mut(&mut self) -> &mut Vec<ScriptAsset> {
        &mut self.scripts
    }

    pub fn set_scripts(&mut self, scripts: Vec<ScriptAsset>) {
        self.scripts = scripts;
    }

    /// Generate a qualified name relative to the plugin name.
    pub fn qualified(&self, val: &str) -> String {
        format!("{}::{}", &self.name, val)
    }

    /// Get the path for the plugin assets.
    pub fn to_assets_path(&self) -> PathBuf {
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

    /// Library license.
    pub license: Option<LicenseGroup>,

    /// Library website.
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub website: Option<Url>,

    /// Library repository.
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub repository: Option<Url>,

}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
