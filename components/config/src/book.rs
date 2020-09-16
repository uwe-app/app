use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

static THEME_NAME: &str = "default";
static THEME_TARGET: &str = "assets/book/theme";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct BookConfig {

    #[serde(flatten)]
    pub members: HashMap<String, BookItem>,

    /// Name of a book theme to use.
    pub theme: Option<String>,

    /// Target output directory.
    #[serde(skip)]
    pub target: Option<PathBuf>,

}

impl Default for BookConfig {
    fn default() -> Self {
        Self {
            theme: Some(THEME_NAME.to_string()),
            members: HashMap::new(),
            target: Some(
                PathBuf::from(
                    utils::url::to_path_separator(THEME_TARGET))),
        }
    }
}

impl BookConfig {

    pub(crate) fn prepare(&mut self) {
        // TODO: define default menu using SUMMARY.md
    }

    pub fn theme_name(&self) -> &str {
        self.theme.as_ref().map(|s| s.as_str()).unwrap_or(THEME_NAME) 
    }

    pub fn target(&self) -> &PathBuf {
        self.target.as_ref().unwrap()
    }

    /// Get a list of paths for all books.
    pub fn get_paths(&self, base: &PathBuf) -> HashMap<&str, PathBuf> {
        let mut out: HashMap<&str, PathBuf> = HashMap::new();
        for (key, value) in self.members.iter() {
            out.insert(key, base.join(&value.path));
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
