use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

static BOOK_TOML: &str = "book.toml";
static DEFAULT_THEME_NAME: &str = "default";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookConfig {
    pub theme: Option<String>,
    #[serde(flatten)]
    pub members: HashMap<String, BookItem>,
}

impl BookConfig {
    pub(crate) fn prepare(&mut self, source: &PathBuf) -> Result<()> {
        let book_paths = self.get_paths(source);
        for mut p in book_paths {
            if !p.exists() || !p.is_dir() {
                return Err(Error::NotDirectory(p));
            }

            p.push(BOOK_TOML);
            if !p.exists() || !p.is_file() {
                return Err(Error::NoBookConfig(p));
            }
        }

        Ok(())
    }

    /// Get a list of paths for all books.
    pub fn get_paths(&self, base: &PathBuf) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        for (_, value) in self.members.iter() {
            out.push(base.join(&value.path));
        }
        out
    }

    /// Find a book by path.
    pub fn find(&self, needle: &PathBuf) -> Option<BookItem> {
        for (_, value) in self.members.iter() {
            if &value.path == needle {
                return Some(value.clone());
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
