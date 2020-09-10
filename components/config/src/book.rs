use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookConfig {
    pub theme: Option<PathBuf>,
    #[serde(flatten)]
    pub members: HashMap<String, HashMap<String, BookItem>>,
}

impl BookConfig {
    pub fn get_paths<P: AsRef<Path>>(&self, base: P) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        let source = base.as_ref().to_path_buf();
        for (_, map) in &self.members {
            for (_, value) in map {
                let mut tmp = source.clone();
                tmp.push(value.path.clone());
                out.push(tmp);
            }
        }
        out
    }

    pub fn find<P: AsRef<Path>>(&self, path: P) -> Option<BookItem> {
        let needle = path.as_ref().to_path_buf();
        for (_, map) in &self.members {
            for (_, value) in map {
                if value.path == needle {
                    return Some(value.clone());
                }
            }
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BookItem {
    pub path: PathBuf,
    pub draft: Option<bool>,
}

