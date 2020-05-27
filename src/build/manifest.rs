use std::path::Path;
use std::convert::AsRef;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value, Map};

#[derive(Serialize, Deserialize)]
pub struct ManifestEntry {
    modified: SystemTime,
}

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub map: Map<String, Value>
}

impl Manifest {
    pub fn new() -> Self {
        Manifest{map: Map::new()}
    }

    fn get_key<P: AsRef<Path>>(&self, file: P) -> String {
        file.as_ref().to_string_lossy().into_owned()
    }

    fn get_entry<P: AsRef<Path>>(&self, file: P, _dest: P) -> Option<ManifestEntry> {
        if let Ok(meta) = file.as_ref().metadata() {
            if let Ok(modified) = meta.modified() {
                return Some(ManifestEntry{
                    modified,
                })
            }
        }
        None
    }

    pub fn is_dirty<P: AsRef<Path>>(&self, file: P, dest: P) -> bool {
        if !dest.as_ref().exists() {
            return true
        }

        let key = self.get_key(file.as_ref());
        if !self.map.contains_key(&key) {
            return true 
        }

        if let Some(entry) = self.map.get(&key) {
            let entry: ManifestEntry = serde_json::from_value(json!(entry)).unwrap();
            if let Some(current) = self.get_entry(file, dest) {
                if current.modified > entry.modified {
                    return true 
                }
            }
        }

        false
    }

    pub fn touch<P: AsRef<Path>>(&mut self, file: P, dest: P) {
        let key = self.get_key(file.as_ref());

        if !file.as_ref().exists() {
            self.map.remove(&key); 
        }

        if let Some(value) = self.get_entry(file.as_ref(), dest.as_ref()) {
            self.map.insert(key, json!(value));
        }
    }
}
