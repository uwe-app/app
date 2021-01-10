use std::collections::{BTreeMap, HashMap};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use crate::{
    dependency::DependencyMap,
    features::FeatureMap,
    plugin::{Plugin, PluginMap},
};

// FIXME: use a technique like ReleaseVersion to order correctly!

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistryEntry {
    #[serde_as(as = "BTreeMap<DisplayFromStr, _>")]
    #[serde(flatten)]
    pub versions: BTreeMap<Version, RegistryItem>,
}

impl RegistryEntry {
    pub fn get(&self, version: &Version) -> Option<&RegistryItem> {
        self.versions.get(version)
    }

    pub fn find(&self, req: &VersionReq) -> Option<(&Version, &RegistryItem)> {
        for (v, item) in self.versions.iter().rev() {
            if req.matches(v) {
                return Some((v, item));
            }
        }
        None
    }

    pub fn all(&self, req: &VersionReq) -> Vec<RegistryItem> {
        let mut out = Vec::new();
        for (v, item) in self.versions.iter() {
            if req.matches(v) {
                out.push(item.clone())
            }
        }
        out
    }

    pub fn latest(&self) -> Option<(&Version, &RegistryItem)> {
        let mut it = self.versions.iter().rev();
        it.next()
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct RegistryItem {
    name: String,

    #[serde_as(as = "DisplayFromStr")]
    version: Version,

    /// Checksum for the compressed archive.
    pub digest: String,

    /// The plugin dependency specifications. We must store these
    /// so the solver can determine nested dependencies before the
    /// plugin has been downloaded and extracted.
    #[serde(skip_serializing_if = "DependencyMap::is_empty")]
    dependencies: DependencyMap,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    plugins: PluginMap,

    /// The feature names that the plugin declares.
    #[serde(skip_serializing_if = "FeatureMap::is_empty")]
    features: FeatureMap,
}

impl Default for RegistryItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.0.0".parse().unwrap(),
            digest: String::new(),
            dependencies: Default::default(),
            plugins: Default::default(),
            features: Default::default(),
        }
    }
}

impl RegistryItem {

    pub fn name(&self) -> &str {
        &self.name 
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn dependencies(&self) -> &DependencyMap {
        &self.dependencies
    }

    pub fn plugins(&self) -> &PluginMap {
        &self.plugins
    }

    pub fn features(&self) -> &FeatureMap {
        &self.features
    }

    pub fn short_name(&self) -> Option<&str> {
        self.name.split(crate::PLUGIN_NS).last()
    }
}

impl From<&Plugin> for RegistryItem {
    fn from(plugin: &Plugin) -> RegistryItem {
        let mut item: RegistryItem = Default::default();
        item.name = plugin.name.clone();
        item.version = plugin.version.clone();

        if !plugin.dependencies().is_empty() {
            item.dependencies = plugin.dependencies().clone();
        }

        if !plugin.plugins().is_empty() {
            item.plugins = plugin.plugins().clone();
        }

        if !plugin.features().is_empty() {
            item.features = plugin.features().clone();
        }
        item
    }
}
