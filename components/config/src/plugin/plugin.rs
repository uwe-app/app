use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use jsonfeed::Author;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use url::Url;

use crate::{
    engine::TemplateEngine, href::UrlPath, license::LicenseGroup,
    script::ScriptAsset, style::StyleAsset, ASSETS, PLUGINS,
};

use crate::{dependency::DependencyMap, features::FeatureMap, Result};

pub type PluginMap = HashMap<String, Plugin>;

#[derive(Debug, Clone)]
pub enum PluginSource {
    File(PathBuf),
    Archive(PathBuf),
    Repo(Url),
    Local(String),
    Registry(Url),
}

impl PluginSource {
    pub fn to_url(&self) -> Result<Url> {
        match *self {
            Self::File(ref path) => {
                let url_target = format!(
                    "{}{}",
                    crate::SCHEME_FILE,
                    utils::url::to_href_separator(path)
                );
                Ok(url_target.parse()?)
            }
            Self::Archive(ref path) => {
                let url_target = format!(
                    "{}{}",
                    crate::SCHEME_TAR_LZMA,
                    utils::url::to_href_separator(path)
                );
                Ok(url_target.parse()?)
            }
            Self::Repo(ref url) | Self::Registry(ref url) => Ok(url.clone()),
            Self::Local(ref name) => {
                // TODO: url encoding the name?
                let url_target = format!("{}{}", crate::SCHEME_PLUGIN, name);
                Ok(url_target.parse()?)
            }
        }
    }
}

/// Hint as to the type of plugin.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum PluginType {
    /// Library plugins may contain assets, icons, fonts,
    /// partials, layouts, scripts, styles or any other files.
    #[serde(rename = "library")]
    Library,
    /// Site plugins can be used as project blueprints.
    #[serde(rename = "site")]
    Site,
}

impl fmt::Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::Library => "library",
            Self::Site => "site",
        })
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

    /// Plugin license.
    license: Option<LicenseGroup>,

    /// Plugin author(s).
    authors: Option<Vec<Author>>,

    /// List of keywords.
    keywords: Option<Vec<String>>,

    /// Type of the plugin.
    #[serde(rename = "type")]
    kind: PluginType,

    /// Prefix for scoped plugins.
    prefix: Option<UrlPath>,

    /// List of remote orgins used by this plugin.
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    origins: Option<Vec<Url>>,

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

    /// Collections of partials and layouts
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    templates: HashMap<TemplateEngine, PluginTemplates>,

    /// Plugin dependencies.
    #[serde(skip_serializing_if = "DependencyMap::is_empty")]
    dependencies: DependencyMap,

    /// Collection of scoped plugins.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    plugins: PluginMap,

    /// Plugin features.
    #[serde(skip_serializing_if = "FeatureMap::is_empty")]
    features: FeatureMap,

    // WARN: the position of this is important. It must be
    // WARN: after plugin assets otherwise we get the TOML
    // WARN: error: `values must be emitted before tables`.
    /// List of third-party libraries the plugin depends on.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    library: HashMap<String, ExternalLibrary>,

    /// Base path this plugin was loaded from,
    /// used to resolve assets during collation.
    #[serde(skip)]
    base: PathBuf,

    /// A checksum digest when extracted from a registry archive.
    #[serde(skip)]
    checksum: Option<String>,

    /// A source URL the plugin was loaded from.
    #[serde(skip)]
    source: Option<PluginSource>,
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
            kind: PluginType::Library,
            origins: None,
            assets: HashSet::new(),
            styles: Vec::new(),
            scripts: Vec::new(),
            templates: HashMap::new(),
            plugins: HashMap::new(),
            dependencies: Default::default(),
            features: Default::default(),
            library: HashMap::new(),
            base: PathBuf::from(String::new()),
            checksum: None,
            source: None,
            prefix: None,
        }
    }
}

impl Plugin {
    pub fn new_scope(parent: &Plugin, name: &str, prefix: UrlPath) -> Self {
        Self {
            name: format!("{}{}{}", &parent.name, crate::PLUGIN_NS, name),
            description: parent.description.clone(),
            version: parent.version.clone(),
            prefix: Some(prefix),
            ..Default::default()
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn kind(&self) -> &PluginType {
        &self.kind 
    }

    pub fn parent(&self) -> String {
        let mut parts = self.name.split(crate::PLUGIN_NS).collect::<Vec<_>>();
        parts.pop();
        parts.join(crate::PLUGIN_NS)
    }

    //pub fn local_name(&self) -> String {
    //let mut parts = self.name.split(crate::PLUGIN_NS).collect::<Vec<_>>();
    //parts.pop()
    //}

    pub fn base(&self) -> &PathBuf {
        &self.base
    }

    pub fn set_base<P: AsRef<Path>>(&mut self, p: P) {
        self.base = p.as_ref().to_path_buf();
    }

    pub fn source(&self) -> &Option<PluginSource> {
        &self.source
    }

    pub fn set_source(&mut self, u: PluginSource) {
        self.source = Some(u);
    }

    pub fn checksum(&self) -> &Option<String> {
        &self.checksum
    }

    pub fn license(&self) -> &Option<LicenseGroup> {
        &self.license
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

    pub fn templates(&self) -> &HashMap<TemplateEngine, PluginTemplates> {
        &self.templates
    }

    pub fn templates_mut(
        &mut self,
    ) -> &mut HashMap<TemplateEngine, PluginTemplates> {
        &mut self.templates
    }

    pub fn features(&self) -> &FeatureMap {
        &self.features
    }

    pub fn features_mut(&mut self) -> &mut FeatureMap {
        &mut self.features
    }

    /// Collection of dependencies.
    pub fn dependencies(&self) -> &DependencyMap {
        &self.dependencies
    }

    /// Mutable collection of dependencies.
    pub fn dependencies_mut(&mut self) -> &mut DependencyMap {
        &mut self.dependencies
    }

    /// Collection of scoped plugins.
    pub fn plugins(&self) -> &HashMap<String, Plugin> {
        &self.plugins
    }

    /// Mutable collection of scoped plugins.
    pub fn plugins_mut(&mut self) -> &mut HashMap<String, Plugin> {
        &mut self.plugins
    }

    pub fn library(&self) -> &HashMap<String, ExternalLibrary> {
        &self.library
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
        self.base.join(utils::url::to_path_separator(
            path.as_str().trim_start_matches("/"),
        ))
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalLibrary {
    /// Library version.
    #[serde_as(as = "DisplayFromStr")]
    version: Version,

    /// Library license.
    license: Option<LicenseGroup>,

    /// Library website.
    #[serde_as(as = "Option<DisplayFromStr>")]
    website: Option<Url>,

    /// Library repository.
    #[serde_as(as = "Option<DisplayFromStr>")]
    repository: Option<Url>,
}

impl ExternalLibrary {
    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn license(&self) -> &Option<LicenseGroup> {
        &self.license
    }

    pub fn website(&self) -> &Option<Url> {
        &self.website
    }

    pub fn repository(&self) -> &Option<Url> {
        &self.repository
    }
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
            self.file.as_str().trim_start_matches("/"),
        ))
    }
}
