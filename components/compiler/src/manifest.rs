use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use config::RuntimeOptions;


use log::debug;

use crate::Error;
use super::BuildContext;

pub struct Manifest {
    file: ManifestFile,
    incremental: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestFile {
    pub map: Map<String, Value>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestEntry {
    modified: SystemTime,
}

impl Manifest {
    pub fn new(options: RuntimeOptions) -> Self {
        let file = ManifestFile { map: Map::new() };
        Manifest {
            file,
            incremental: options.settings.is_incremental(),
        }
    }

    fn get_key<P: AsRef<Path>>(&self, file: P) -> String {
        file.as_ref().to_string_lossy().into_owned()
    }

    fn get_entry<P: AsRef<Path>>(&self, file: P, _dest: P) -> Option<ManifestEntry> {
        if let Ok(meta) = file.as_ref().metadata() {
            if let Ok(modified) = meta.modified() {
                return Some(ManifestEntry { modified });
            }
        }
        None
    }

    pub fn is_dirty<P: AsRef<Path>>(&self, file: P, dest: P, force: bool) -> bool {
        if !self.incremental || force || !dest.as_ref().exists() {
            return true;
        }

        let key = self.get_key(file.as_ref());
        if !self.file.map.contains_key(&key) {
            return true;
        }

        if let Some(entry) = self.file.map.get(&key) {
            let entry: ManifestEntry = serde_json::from_value(json!(entry)).unwrap();
            if let Some(current) = self.get_entry(file, dest) {
                if current.modified > entry.modified {
                    return true;
                }
            }
        }

        false
    }

    pub fn touch<P: AsRef<Path>>(&mut self, file: P, dest: P) {
        let key = self.get_key(file.as_ref());

        if !file.as_ref().exists() {
            self.file.map.remove(&key);
        }

        if let Some(value) = self.get_entry(file.as_ref(), dest.as_ref()) {
            self.file.map.insert(key, json!(value));
        }
    }

    fn get_manifest_file(&self, ctx: &BuildContext) -> PathBuf {
        let mut file = ctx.options.target.clone();
        let name = file
            .file_name()
            .unwrap_or(std::ffi::OsStr::new(""))
            .to_string_lossy()
            .into_owned();
        if !name.is_empty() {
            file.set_file_name(format!("{}.json", name));
        }
        file
    }

    pub fn load(&mut self, ctx: &BuildContext) -> Result<(), Error> {
        let file = self.get_manifest_file(ctx);
        if file.exists() && file.is_file() {
            debug!("manifest {}", file.display());
            let json = utils::fs::read_string(file)?;
            self.file = serde_json::from_str(&json)?;
        }
        Ok(())
    }

    pub fn save(&self, ctx: &BuildContext) -> Result<(), Error> {
        if ctx.options.settings.is_incremental() {
            let file = self.get_manifest_file(ctx);
            let json = serde_json::to_string(&self.file)?;
            debug!("manifest {}", file.display());
            utils::fs::write_string(file, json)?;
        }
        Ok(())
    }
}
