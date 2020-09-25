use std::path::PathBuf;

use async_trait::async_trait;

use config::plugin::{Plugin, RegistryEntry, RegistryItem};

use crate::{Error, Result};

/// Defines the contract for plugin registry implementations.
#[async_trait]
pub trait RegistryAccess {
    async fn entry(&self, name: &str) -> Result<Option<RegistryEntry>>;
    async fn register(
        &self,
        entry: &mut RegistryEntry,
        plugin: &Plugin,
        digest: &Vec<u8>,
    ) -> Result<PathBuf>;
}

/// Access a registry using a file system backing store.
///
/// Uses separate paths for reading and writing so that during
/// development we can use a local file system path other than
/// the public repository path used for reading.
pub struct RegistryFileAccess {
    pub reader: PathBuf,
    pub writer: PathBuf,
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
    async fn entry(&self, name: &str) -> Result<Option<RegistryEntry>> {
        let mut file_path = self.reader.join(name);
        file_path.set_extension(config::JSON);
        if file_path.exists() {
            let contents = utils::fs::read_string(file_path)?;
            let registry_entry = serde_json::from_str(&contents)?;
            return Ok(Some(registry_entry));
        }
        Ok(None)
    }

    async fn register(
        &self,
        entry: &mut RegistryEntry,
        plugin: &Plugin,
        digest: &Vec<u8>,
    ) -> Result<PathBuf> {
        let mut item = RegistryItem::from(plugin);
        item.digest = hex::encode(digest);

        let version = plugin.version.clone();
        entry.versions.entry(version).or_insert(item);

        let content = serde_json::to_string(entry)?;

        let mut file_path = self.writer.join(&plugin.name);
        file_path.set_extension(config::JSON);
        utils::fs::write_string(&file_path, content)?;

        Ok(file_path)
    }
}
