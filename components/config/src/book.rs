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
    pub fn theme_name(&self) -> &str {
        self.theme.as_ref().map(|s| s.as_str()).unwrap_or(THEME_NAME) 
    }

    pub fn target(&self) -> &PathBuf {
        self.target.as_ref().unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BookItem {
    pub path: PathBuf,
    pub draft: Option<bool>,
}
