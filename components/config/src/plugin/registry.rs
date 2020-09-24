use std::collections::HashMap;

use semver::Version;
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, DisplayFromStr};

use crate::plugin::Plugin;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistryEntry {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    #[serde(flatten)]
    pub versions: HashMap<Version, RegistryItem>,
}

impl RegistryEntry {
    pub fn get(&self, version: &Version) -> Option<&RegistryItem> {
        self.versions.get(version) 
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
