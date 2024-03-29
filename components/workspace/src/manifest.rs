use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    #[serde(skip)]
    pub file: PathBuf,
    map: HashMap<PathBuf, SystemTime>,
}

impl Manifest {
    pub fn new(file: PathBuf) -> Self {
        Manifest {
            file,
            map: HashMap::new(),
        }
    }

    fn get_entry<P: AsRef<Path>>(&self, file: P) -> Option<SystemTime> {
        if let Ok(meta) = file.as_ref().metadata() {
            if let Ok(modified) = meta.modified() {
                return Some(modified);
            }
        }
        None
    }

    pub fn is_dirty(
        &self,
        file: &PathBuf,
        dest: &PathBuf,
        force: bool,
    ) -> bool {
        if force || !dest.exists() {
            return true;
        }
        if let (Some(entry), Some(current)) =
            (self.map.get(file), self.get_entry(file))
        {
            return &current > entry;
        }
        false
    }

    pub fn exists(&self, file: &PathBuf) -> bool {
        self.map.contains_key(file)
    }

    pub fn touch(&mut self, file: &PathBuf) {
        if let Some(value) = self.get_entry(file) {
            self.map.insert(file.to_path_buf(), value);
        }
    }

    pub fn update(&mut self, files: &Vec<Arc<PathBuf>>) {
        for f in files {
            self.touch(&f);
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
