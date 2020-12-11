use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{utils::href::UrlPath, Error, Result};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum MenuVariant {
    /// Automatically create menus with descriptions.
    Description { description: bool, suffix: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MenuConfig {
    /// Variants are automatically created variations on a menu.
    pub variants: Option<HashSet<MenuVariant>>,

    #[serde(flatten)]
    pub entries: HashMap<String, MenuEntry>,
}

impl MenuConfig {
    /// Iterate the entries and create variants.
    pub fn prepare(&mut self) {
        if let Some(ref variants) = self.variants {
            let variants = variants.clone();
            let entries = self.entries.clone();
            for var in variants.into_iter() {
                match var {
                    MenuVariant::Description { ref suffix, .. } => {
                        for (k, v) in entries.iter() {
                            let mut def = v.definition.clone();
                            def.set_description(true);
                            let key = format!("{}{}", k, suffix);
                            self.entries
                                .entry(key)
                                .or_insert(MenuEntry::new_reference(def));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct MenuResult {
    /// Compiled HTML string, may contain template statements.
    pub value: String,

    /// List of pages that were referenced by this menu so that
    /// callers can easily iterate the page data for a menu.
    pub pages: Vec<Arc<String>>,
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
    //pub fn new(name: String, file: UrlPath) -> Self {
        //Self {
            //name,
            //definition: MenuReference::File { file },
            //result: Default::default(),
        //}
    //}

    pub fn new_reference(definition: MenuReference) -> Self {
        Self {
            definition,
            name: Default::default(),
            result: Default::default(),
        }
    }

    /*
    pub fn verify_files(&self, base: &PathBuf) -> Result<()> {
        match self.definition {
            MenuReference::File { ref file, .. } => {
                let buf = base.join(utils::url::to_path_separator(
                    file.as_str().trim_start_matches("/"),
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
    */
}

/// References the definition of a menu.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
#[serde(untagged)]
pub enum MenuReference {
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
        include_index: Option<bool>,
    },
}

impl MenuReference {
    pub fn set_description(&mut self, value: bool) {
        match *self {
            Self::Pages {
                ref mut description,
                ..
            } => *description = Some(value),
            Self::Directory {
                ref mut description,
                ..
            } => *description = Some(value),
        }
    }
}

impl Default for MenuReference {
    fn default() -> Self {
        Self::Pages {
            pages: Vec::new(),
            description: None,
        }
    }
}
