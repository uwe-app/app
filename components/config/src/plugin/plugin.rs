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

    // WARN: the position of this is important. It must be
    // WARN: before the `library` otherwise we get the TOML
    // WARN: error: `values must be emitted before tables`.
    #[serde(flatten)]
    scope: PluginScope,

    /// Collection of scoped plugins.
    plugins: HashMap<String, PluginScope>,

    /// Collection of features for this plugin.
    pub features: Option<FeatureMap>,

    /// Plugin dependencies.
    pub dependencies: Option<DependencyMap>,

    /// List of third-party libraries the plugin depends on.
    pub library: Option<HashMap<String, ExternalLibrary>>,

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
            scope: Default::default(),
            plugins: HashMap::new(),
            dependencies: None,
            features: None,
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
        &self.scope.assets
    }

    pub fn set_assets(&mut self, assets: HashSet<UrlPath>) {
        self.scope.assets = assets;
    }

    pub fn styles(&self) -> &Vec<StyleAsset> {
        &self.scope.styles
    }

    pub fn styles_mut(&mut self) -> &mut Vec<StyleAsset> {
        &mut self.scope.styles
    }

    pub fn set_styles(&mut self, styles: Vec<StyleAsset>) {
        self.scope.styles = styles;
    }

    pub fn scripts(&self) -> &Vec<ScriptAsset> {
        &self.scope.scripts
    }

    pub fn scripts_mut(&mut self) -> &mut Vec<ScriptAsset> {
        &mut self.scope.scripts
    }

    pub fn set_scripts(&mut self, scripts: Vec<ScriptAsset>) {
        self.scope.scripts = scripts;
    }

    pub fn hooks(&self) -> &HookMap {
        &self.scope.hooks
    }

    pub fn hooks_mut(&mut self) -> &mut HookMap {
        &mut self.scope.hooks
    }

    pub fn templates(&self) -> &HashMap<TemplateEngine, PluginTemplates> {
        &self.scope.templates
    }

    pub fn templates_mut(&mut self) -> &mut HashMap<TemplateEngine, PluginTemplates> {
        &mut self.scope.templates
    }

    pub fn plugins(&self) -> &HashMap<String, PluginScope> {
        &self.plugins
    }

    pub fn plugins_mut(&mut self) -> &mut HashMap<String, PluginScope> {
        &mut self.plugins
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

/// A plugin scope encapsulates the references for a plugin
/// that the application can action (as opposed to plugin
/// meta data).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PluginScope {
    /// List of synthetic assets to include in the project.
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    assets: HashSet<UrlPath>,

    // NOTE: we want to use HashSet for styles and scripts
    // NOTE: so there are no duplicates but ordering is important
    // NOTE: for these types so we just use a Vec for now.

    /// List of stylesheets to add to pages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    styles: Vec<StyleAsset>,

    /// List of scripts to add to pages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scripts: Vec<ScriptAsset>,

    /// List of hooks in this plugin.
    #[serde(skip_serializing_if = "HookMap::is_empty")]
    hooks: HookMap,

    /// Collections of partials and layouts
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    templates: HashMap<TemplateEngine, PluginTemplates>,
}

impl Default for PluginScope {
    fn default() -> Self {
        Self {
            assets: HashSet::new(),
            styles: Vec::new(),
            scripts: Vec::new(),
            hooks: Default::default(),
            templates: HashMap::new(),
        }
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
