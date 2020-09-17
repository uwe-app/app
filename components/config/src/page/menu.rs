use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{utils::href::UrlPath, Error, Result};

pub static MENU: &str = "MENU.md";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MenuConfig {
    #[serde(flatten)]
    pub entries: HashMap<String, MenuEntry>,
}

#[derive(Default, Debug)]
pub struct MenuResult {
    pub value: String,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq)]
pub struct MenuEntry {
    #[serde(flatten, skip_serializing)]
    pub definition: MenuReference,

    /// Stores the hash map key as the name so that after
    /// the menu is compiled it can be re-assigned to the
    /// correct page menu entry.
    #[serde(skip)]
    pub name: String,

    /// The compiled menu as HTML but before template parsing.
    #[serde(skip)]
    pub result: String,
}

impl MenuEntry {

    pub fn new(name: String, file: UrlPath, relative: bool) -> Self {
        Self {
            name,
            definition: MenuReference::File {file, relative},
            result: Default::default(),
        }
    }

    /// Determine if a file appears to be a menu using the convention.
    pub fn is_menu(file: &PathBuf) -> bool {
        file.ends_with(MENU) 
    }

    pub fn verify_files(&self, base: &PathBuf) -> Result<()> {
        match self.definition {
            MenuReference::File { ref file, .. } => {
                let buf = base.join(utils::url::to_path_separator(
                    file.trim_start_matches("/"),
                ));
                if !buf.exists() || !buf.is_file() {
                    return Err(Error::NoMenuFile(file.to_string(), buf));
                }
            }
            // NOTE: other variants must be verified elsewhere once
            // NOTE: we have the collation data
            _ => {}
        }
        Ok(())
    }
}

/// References the definition of a menu.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
#[serde(untagged)]
pub enum MenuReference {
    /// Render the context of a template file as the menu.
    File { file: UrlPath, relative: bool },

    /// Render a collection of specific pages.
    Pages {
        pages: Vec<UrlPath>,
        description: Option<bool>,
    },

    /// Render all the pages starting with the given directory.
    Directory {
        directory: UrlPath,
        description: Option<bool>,
        depth: Option<usize>,
    },
}

impl Default for MenuReference {
    fn default() -> Self {
        Self::Pages {
            pages: Vec::new(),
            description: None,
        }
    }
}
