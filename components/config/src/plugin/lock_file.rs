use std::collections::hash_map::RandomState;
use std::collections::hash_set::Difference;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use url::Url;

use crate::Result;

pub type PackageSet = HashSet<LockFileEntry>;

#[derive(Debug, Serialize, Deserialize, Clone, Default, Eq, PartialEq)]
#[serde(default)]
pub struct LockFile {
    pub package: PackageSet,
}

impl LockFile {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let lock_file: LockFile = if path.exists() && path.is_file() {
            let contents = utils::fs::read_string(path)?;
            toml::from_str(&contents)?
        } else {
            Default::default()
        };
        Ok(lock_file)
    }

    pub fn get_lock_file<P: AsRef<Path>>(base: P) -> PathBuf {
        base.as_ref().join(crate::SITE_LOCK)
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = toml::to_string(self)?;
        utils::fs::write_string(path, content)?;
        Ok(())
    }

    pub fn diff<'a>(
        &'a self,
        other: &'a LockFile,
    ) -> Difference<'a, LockFileEntry, RandomState> {
        self.package.difference(&other.package)
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(default)]
pub struct LockFileEntry {
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub source: Option<Url>,
    pub checksum: Option<String>,
    pub dependencies: Option<Vec<String>>,
}

impl Default for LockFileEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.0.0".parse().unwrap(),
            source: None,
            checksum: None,
            dependencies: None,
        }
    }
}

//impl From<&Plugin> for LockFileEntry {
//fn from(plugin: &Plugin) -> Self {
//Self {
//name: plugin.name.clone(),
//version: plugin.version.clone(),
//source: plugin.source.clone(),
//checksum: plugin.checksum.clone(),
//dependencies: None,
//}
//}
//}
