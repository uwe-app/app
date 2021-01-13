use std::collections::{BTreeMap, HashMap};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use crate::{
    dependency::DependencyMap,
    features::FeatureMap,
    plugin::{Plugin, PluginMap, VersionKey},
};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistryEntry {
    #[serde_as(as = "BTreeMap<DisplayFromStr, _>")]
    #[serde(flatten)]
    pub versions: BTreeMap<VersionKey, RegistryItem>,
}

impl RegistryEntry {
    pub fn versions(&self) -> &BTreeMap<VersionKey, RegistryItem> {
        &self.versions
    }

    pub fn get(&self, version: &Version) -> Option<&RegistryItem> {
        self.versions.get(&VersionKey::from(version))
    }

    pub fn find(&self, req: &VersionReq) -> Option<(&Version, &RegistryItem)> {
        for (v, item) in self.versions.iter() {
            if req.matches(v.semver()) {
                return Some((v.semver(), item));
            }
        }
        None
    }

    pub fn all(&self, req: &VersionReq) -> Vec<RegistryItem> {
        let mut out = Vec::new();
        for (v, item) in self.versions.iter() {
            if req.matches(v.semver()) {
                out.push(item.clone())
            }
        }
        out
    }

    pub fn latest(&self) -> Option<(&VersionKey, &RegistryItem)> {
        let mut it = self.versions.iter();
        it.next()
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
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
            version: Version::new(0, 0, 0),
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
        item.name = plugin.name().to_string();
        item.version = plugin.version().clone();

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
