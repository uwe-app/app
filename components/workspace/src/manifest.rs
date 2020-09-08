use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManifestEntry {
    modified: SystemTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    #[serde(skip)]
    pub file: PathBuf,
    pub map: HashMap<PathBuf, ManifestEntry>,
}

impl Manifest {
    pub fn new(file: PathBuf) -> Self {
        Manifest {
            file,
            map: HashMap::new(),
        }
    }

    fn get_entry<P: AsRef<Path>>(&self, file: P) -> Option<ManifestEntry> {
        if let Ok(meta) = file.as_ref().metadata() {
            if let Ok(modified) = meta.modified() {
                return Some(ManifestEntry { modified });
            }
        }
        None
    }

    pub fn is_dirty<P: AsRef<Path>, D: AsRef<Path>>(
        &self,
        file: P,
        dest: D,
        force: bool,
    ) -> bool {
        if force || !dest.as_ref().exists() {
            return true;
        }
        if let Some(entry) = self.map.get(&file.as_ref().to_path_buf()) {
            if let Some(current) = self.get_entry(file) {
                if current.modified > entry.modified {
                    return true;
                }
            }
        }
        false
    }

    pub fn exists<P: AsRef<Path>>(&self, file: P) -> bool {
        return self.map.contains_key(&file.as_ref().to_path_buf());
    }

    pub fn touch<P: AsRef<Path>>(&mut self, file: P) {
        if let Some(value) = self.get_entry(file.as_ref()) {
            self.map.insert(file.as_ref().to_path_buf(), value);
        }
    }

    pub fn load<P: AsRef<Path>>(p: P) -> Result<Manifest> {
        let file = p.as_ref();
        if file.exists() && file.is_file() {
            let mut manifest: Manifest =
                serde_json::from_str(&utils::fs::read_string(file)?)?;
            manifest.file = file.to_path_buf();
            return Ok(manifest);
        }
        Ok(Manifest::new(file.to_path_buf()))
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(self)?;
        utils::fs::write_string(&self.file, json)?;
        Ok(())
    }
}
