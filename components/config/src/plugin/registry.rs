use std::collections::BTreeMap;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use crate::{plugin::Plugin, dependency::{DependencyRef, DependencyMap, Dependency}};

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

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistryItem {
    pub name: String,

    /// Checksum for the compressed archive.
    pub digest: String,

    /// The plugin dependency specifications. We must store these 
    /// so the solver can determine nested dependencies before the 
    /// plugin has been downloaded and extracted.
    pub dependencies: Option<Vec<DependencyRef>>,

    /// The plugin dependencies which are optional.
    pub optional: Option<Vec<String>>,

    /// The feature names that the plugin declares.
    pub features: Option<Vec<String>>,
}

impl RegistryItem {
    pub fn to_dependency_map(&self) -> DependencyMap {
        if let Some(ref dependencies) = self.dependencies {
            let mut map: DependencyMap = Default::default(); 
            dependencies
                .iter()
                .for_each(|dep_ref| {
                    let dep = Dependency::from(dep_ref);
                    map.items.insert(dep_ref.name.clone(), dep);
                });
            return map
        }
        Default::default()
    }
}

impl From<&Plugin> for RegistryItem {
    fn from(plugin: &Plugin) -> RegistryItem {
        let mut item: RegistryItem = Default::default();
        item.name = plugin.name.clone();

        if let Some(ref dependencies) = plugin.dependencies {
            let deps: Vec<DependencyRef> = dependencies
                .iter()
                .map(DependencyRef::from)
                .collect();
            item.dependencies = Some(deps);

            let optional: Vec<String> = dependencies
                .iter()
                .filter(|(_, d)| d.optional.is_some() && d.optional.unwrap())
                .map(|(_, d)| d.name.as_ref().unwrap().clone())
                .collect();
            item.optional = Some(optional);
        }

        if let Some(ref features) = plugin.features {
            let features: Vec<String> = features
                .keys()
                .map(|k| k.to_string())
                .collect();
            item.features = Some(features);
        }

        item
    }
}
