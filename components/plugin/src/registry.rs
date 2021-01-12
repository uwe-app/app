use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use async_trait::async_trait;

use semver::{Version, VersionReq};

use config::{
    registry::{RegistryEntry, RegistryItem},
    Plugin, PluginSpec,
};

use crate::{Error, Registry, Result};

/// Defines the contract for plugin registry implementations.
#[async_trait]
pub trait RegistryAccess {
    async fn resolve(
        &self,
        name: &str,
        version: &VersionReq,
    ) -> Result<(Version, RegistryItem)>;

    async fn entry(&self, name: &str) -> Result<Option<RegistryEntry>>;
    async fn spec(&self, spec: &PluginSpec) -> Result<Option<RegistryItem>>;
    async fn find(&self, spec: &PluginSpec) -> Result<Vec<RegistryItem>>;

    /// Find all the plugins whose fully qualified name starts with the needle.
    async fn starts_with(
        &self,
        needle: &str,
    ) -> Result<BTreeMap<String, RegistryEntry>>;

    async fn register(
        &self,
        entry: &mut RegistryEntry,
        plugin: &Plugin,
        digest: &Vec<u8>,
    ) -> Result<PathBuf>;

    async fn read_file(&self, file: &PathBuf) -> Result<RegistryEntry>;
}

/// Access a registry using a file system backing store.
///
/// Uses separate paths for reading and writing so that during
/// development we can use a local file system path other than
/// the public repository path used for reading.
pub struct RegistryFileAccess {
    reader: PathBuf,
    writer: PathBuf,
}

impl RegistryFileAccess {
    pub fn new(reader: PathBuf, writer: PathBuf) -> Result<Self> {
        if !reader.exists() || !reader.is_dir() {
            return Err(Error::RegistryNotDirectory(reader));
        }

        if !writer.exists() || !writer.is_dir() {
            return Err(Error::RegistryNotDirectory(writer));
        }

        Ok(Self { reader, writer })
    }
}

#[async_trait]
impl RegistryAccess for RegistryFileAccess {

    async fn resolve(
        &self,
        name: &str,
        version: &VersionReq,
    ) -> Result<(Version, RegistryItem)> {
        let entry = self
            .entry(name)
            .await?
            .ok_or_else(|| Error::RegistryPackageNotFound(name.to_string()))?;

        let (version, package) = entry.find(version).ok_or_else(|| {
            Error::RegistryPackageVersionNotFound(
                name.to_string(),
                version.to_string(),
            )
        })?;

        Ok((version.clone(), package.clone()))
    }

    async fn read_file(&self, file: &PathBuf) -> Result<RegistryEntry> {
        let contents = utils::fs::read_string(file)?;
        Ok(serde_json::from_str(&contents)
            .map_err(|e| Error::RegistryParse(file.to_path_buf(), e.to_string()))?)
    }

    async fn entry(&self, name: &str) -> Result<Option<RegistryEntry>> {
        let mut file_path = self.reader.join(name);
        file_path.set_extension(config::JSON);
        if file_path.exists() {
            return Ok(Some(self.read_file(&file_path).await?));
        }
        Ok(None)
    }

    async fn spec(&self, spec: &PluginSpec) -> Result<Option<RegistryItem>> {
        let name = spec.name();
        let range = spec.range();
        if let Some(entry) = self.entry(name).await? {
            if let Some((_, item)) = entry.find(range) {
                return Ok(Some(item.clone()));
            }
        }
        Ok(None)
    }

    async fn find(&self, spec: &PluginSpec) -> Result<Vec<RegistryItem>> {
        let name = spec.name();
        let range = spec.range();
        if let Some(entry) = self.entry(name).await? {
            return Ok(entry.all(range));
        }
        Ok(vec![])
    }

    async fn starts_with(
        &self,
        needle: &str,
    ) -> Result<BTreeMap<String, RegistryEntry>> {
        let mut map = BTreeMap::new();
        for entry in fs::read_dir(&self.reader)? {
            let path = entry?.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if file_name.to_string_lossy().starts_with(needle) {
                        let file_path = path.to_path_buf();
                        let registry_entry = self.read_file(&file_path).await?;
                        let plugin_name = file_path
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        map.insert(plugin_name, registry_entry);
                    }
                }
            }
        }
        Ok(map)
    }

    async fn register(
        &self,
        entry: &mut RegistryEntry,
        plugin: &Plugin,
        digest: &Vec<u8>,
    ) -> Result<PathBuf> {
        let mut item = RegistryItem::from(plugin);
        item.digest = hex::encode(digest);

        let version = plugin.version().clone();
        entry.versions.entry(version).or_insert(item);

        let content = serde_json::to_string(entry)?;

        let mut file_path = self.writer.join(&plugin.name);
        file_path.set_extension(config::JSON);
        utils::fs::write_string(&file_path, content)?;

        Ok(file_path)
    }
}

pub fn new_registry<'r>() -> Result<Registry<'r>> {
    let reg = dirs::packages_dir()?;
    Ok(Box::new(RegistryFileAccess::new(reg.clone(), reg.clone())?))
}
