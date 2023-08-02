use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use async_trait::async_trait;
use semver::{Version, VersionReq};

use config::{
    registry::{RegistryEntry, RegistryItem},
    Plugin, PluginSpec, VersionKey,
};

use crate::{Error, Registry, Result};

pub async fn check_for_updates() -> Result<bool> {
    let registry_repo = dirs::registry_dir()?;
    let repo = scm::open(&registry_repo)?;
    let (is_current, _) = scm::is_current_with_remote(&repo, None, None)?;
    Ok(is_current)
}

pub async fn update_registry() -> Result<()> {
    scm::system_repo::fetch_registry().await?;
    Ok(())
}

/// Defines the contract for plugin registry implementations.
#[async_trait]
pub trait RegistryAccess {
    /// Load all registry packages into memory.
    async fn all(&self) -> Result<BTreeMap<String, RegistryEntry>>;

    /// Try to resolve a registry item.
    async fn resolve(
        &self,
        name: &str,
        version: &VersionReq,
    ) -> Result<(Version, RegistryItem)>;

    /// Load a single registry entry.
    async fn entry(&self, name: &str) -> Result<Option<RegistryEntry>>;

    /// Try to find a single registry item matching the given plugin spec.
    async fn spec(&self, spec: &PluginSpec) -> Result<Option<RegistryItem>>;

    /// Try to find all registry items matching the given plugin spec.
    async fn find(&self, spec: &PluginSpec) -> Result<Vec<RegistryItem>>;

    /// Find all the plugins whose fully qualified name starts with the needle.
    async fn starts_with(
        &self,
        needle: &str,
    ) -> Result<BTreeMap<String, RegistryEntry>>;

    /// Register a plugin by writing the package registry file to disc
    /// for the writer side of this registry.
    ///
    /// The digest should be the checksum for the archive that bundles
    /// the plugin files.
    async fn register(
        &self,
        entry: &mut RegistryEntry,
        plugin: &Plugin,
        digest: &Vec<u8>,
    ) -> Result<PathBuf>;

    /// Read a registry package definition.
    async fn read_file(&self, file: &PathBuf) -> Result<RegistryEntry>;

    /// Find all the versions that are installed for a registry entry.
    async fn installed_versions(
        &self,
        entry: &RegistryEntry,
    ) -> Result<BTreeMap<VersionKey, RegistryItem>>;
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
    async fn all(&self) -> Result<BTreeMap<String, RegistryEntry>> {
        let mut out = BTreeMap::new();
        for entry in fs::read_dir(&self.reader)? {
            let path = entry?.path().to_path_buf();
            if path.is_file() {
                let registry_entry = self.read_file(&path).await?;
                if let Some(stem) = path.file_stem() {
                    let name = stem.to_string_lossy().into_owned().to_string();
                    out.insert(name, registry_entry);
                }
            }
        }
        Ok(out)
    }

    async fn installed_versions(
        &self,
        entry: &RegistryEntry,
    ) -> Result<BTreeMap<VersionKey, RegistryItem>> {
        let mut out = BTreeMap::new();
        for (version, item) in entry.versions() {
            let installation =
                crate::installation_dir(item.name(), version.semver())?;
            if installation.exists() && installation.is_dir() {
                out.insert(version.clone(), item.clone());
            }
        }
        Ok(out)
    }

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

        println!("resolving version {:#?}", version);
        println!("got package {:#?}", package);

        Ok((version.clone(), package.clone()))
    }

    async fn read_file(&self, file: &PathBuf) -> Result<RegistryEntry> {
        let contents = utils::fs::read_string(file)?;
        Ok(serde_json::from_str(&contents).map_err(|e| {
            Error::RegistryParse(file.to_path_buf(), e.to_string())
        })?)
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

        //let version = plugin.version().clone();
        entry
            .versions
            .entry(VersionKey::from(plugin.version()))
            .or_insert(item);

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
