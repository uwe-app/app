use std::collections::hash_map;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fmt;

use globset::{Glob, GlobMatcher};
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
    ASSETS, PLUGINS,
};

// TODO: spdx license for Plugin and ExternalLibrary

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

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    /// Required version for the dependency.
    #[serde_as(as = "DisplayFromStr")]
    pub version: VersionReq,

    /// Path for a local file system plugin.
    pub path: Option<PathBuf>,

    /// Patterns that determine how styles, scripts and layouts 
    /// are applied to pages.
    pub apply: Option<Apply>,

    /// Resolved plugin for this dependency.
    #[serde(skip)]
    pub plugin: Option<Plugin>,

    /// Injected when resolving dependencies from the hash map key.
    #[serde(skip)]
    pub name: Option<String>,
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name.as_ref().unwrap(), self.version.to_string())
    }
}

impl Dependency {
    /// Cache glob patterns used to apply plugins to
    /// files.
    pub fn prepare(&mut self) -> Result<()> {
        if let Some(ref mut apply) = self.apply {
            apply.prepare()?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Apply {
    pub styles: Option<Vec<Glob>>,
    pub scripts: Option<Vec<Glob>>,
    pub layouts: Option<HashMap<String, Vec<Glob>>>,

    #[serde(skip)]
    pub styles_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub scripts_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub layouts_match: HashMap<String, Vec<GlobMatcher>>,
}

impl Apply {

    /// Prepare the global patterns by compiling them.
    ///
    /// Original GlobSet declarations are moved out of the Option(s).
    pub(crate) fn prepare(&mut self) -> Result<()> {
        self.styles_match = if let Some(styles) = self.styles.take() {
            styles.iter().map(|g| g.compile_matcher()).collect()
        } else { Vec::new() };

        self.scripts_match = if let Some(scripts) = self.scripts.take() {
            scripts.iter().map(|g| g.compile_matcher()).collect()
        } else { Vec::new() };

        self.layouts_match = if let Some(layouts) = self.layouts.take() {
            let mut tmp: HashMap<String, Vec<GlobMatcher>> = HashMap::new();
            for (k, v) in layouts {
                tmp.insert(k, v.iter().map(|g| g.compile_matcher()).collect());
            }
            tmp
        } else { HashMap::new() };
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

    /// List of third-party libraries the plugin depends on.
    pub library: Option<Vec<ExternalLibrary>>,

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
            library: None,
            base: PathBuf::from(String::new()),
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
        base.join(
            utils::url::to_path_separator(
                self.file.trim_start_matches("/")))
    }
}
