use std::convert::{TryFrom, TryInto};
use std::path::{Path, PathBuf};

use indexmap::IndexSet;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use url::Url;

use crate::Result;

use super::plugin::Plugin;

#[derive(Debug, Serialize, Deserialize, Clone, Default, Eq, PartialEq)]
#[serde(default)]
pub struct LockFile {
    pub package: IndexSet<LockFileEntry>,
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

    pub fn union(old: LockFile, new: LockFile) -> LockFile {
        let package: IndexSet<LockFileEntry> =
            old.package.union(&new.package).cloned().collect();
        LockFile { package }
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash)]
#[serde(default)]
pub struct LockFileEntry {
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,
    pub checksum: Option<String>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub source: Option<Url>,
}

impl PartialEq for LockFileEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl TryFrom<&Plugin> for LockFileEntry {
    type Error = crate::Error;

    fn try_from(
        plugin: &Plugin,
    ) -> std::result::Result<LockFileEntry, Self::Error> {
        let mut entry: LockFileEntry = Default::default();
        entry.name = plugin.name().to_string();
        entry.version = plugin.version().clone();
        entry.checksum = plugin.checksum().clone();

        if let Some(source) = plugin.source() {
            entry.source = Some(source.clone().try_into()?)
        }

        Ok(entry)
    }
}

impl Default for LockFileEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: Version::new(0, 0, 0),
            source: None,
            checksum: None,
            //dependencies: None,
        }
    }
}

impl LockFileEntry {
    pub fn version(&self) -> &Version {
        &self.version
    }
}
