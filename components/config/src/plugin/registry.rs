use std::collections::BTreeMap;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use crate::{dependency::DependencyMap, features::FeatureMap, plugin::Plugin};

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
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryItem {
    pub name: String,

    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,

    /// Checksum for the compressed archive.
    pub digest: String,

    /// The plugin dependency specifications. We must store these
    /// so the solver can determine nested dependencies before the
    /// plugin has been downloaded and extracted.
    pub dependencies: Option<DependencyMap>,

    /// The feature names that the plugin declares.
    pub features: Option<FeatureMap>,
}

impl Default for RegistryItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.0.0".parse().unwrap(),
            digest: String::new(),
            dependencies: None,
            features: None,
        }
    }
}

impl RegistryItem {
    pub fn to_dependency_map(&self) -> DependencyMap {
        if let Some(ref dependencies) = self.dependencies {
            return dependencies.clone();
        }
        Default::default()
    }
}

impl From<&Plugin> for RegistryItem {
    fn from(plugin: &Plugin) -> RegistryItem {
        let mut item: RegistryItem = Default::default();
        item.name = plugin.name.clone();
        item.version = plugin.version.clone();
        if let Some(ref dependencies) = plugin.dependencies {
            item.dependencies = Some(dependencies.clone());
        }
        if let Some(ref features) = plugin.features {
            item.features = Some(features.clone());
        }
        item
    }
}
