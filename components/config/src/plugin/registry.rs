use std::collections::HashMap;

use semver::Version;
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistryEntry {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub versions: HashMap<Version, RegistryItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistryItem;
