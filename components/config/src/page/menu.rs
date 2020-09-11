use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    Error,
    Result,
    utils::href::UrlPath,
};

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MenuEntry {
    #[serde(flatten, skip_serializing)]
    pub definition: MenuReference,
    #[serde(skip_deserializing)]
    pub result: &'static str,
}

impl MenuEntry {
    pub fn verify_files(&self, base: &PathBuf) -> Result<()> {
        match self.definition {
            MenuReference::File { ref file } => {
                let buf = base.join(
                    utils::url::to_path_separator(
                        file.trim_start_matches("/")));
                if !buf.exists() || !buf.is_file() {
                    return Err(Error::NoMenuFile(file.to_string(), buf))           
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
    File{ file: UrlPath },
    Pages{ pages: Vec<UrlPath> },
}

impl Default for MenuReference {
    fn default() -> Self {
        Self::Pages {pages: Vec::new()}
    }
}
