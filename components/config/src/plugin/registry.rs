use std::collections::BTreeMap;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, DisplayFromStr};

use crate::plugin::Plugin;

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

    pub fn find(&self, req: &VersionReq) -> Option<&RegistryItem> {
        for (v, item) in self.versions.iter().rev() {
            if req.matches(v) {
                return Some(item); 
            }
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistryItem {
    pub name: String,
    pub digest: String,
}

impl From<&Plugin> for RegistryItem {
    fn from(plugin: &Plugin) -> RegistryItem {
        let mut item: RegistryItem = Default::default();
        item.name = plugin.name.clone();
        item
    }
}
