use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Error, Result};

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LinkConfig {
    /// Explicit list of paths that are allowed, should
    /// not begin with a forward slash
    pub allow: Option<Vec<String>>,
    /// The link helper should verify links
    pub verify: Option<bool>,
    /// The link helper should make links relative
    pub relative: Option<bool>,
    /// Catalog for markdown documents
    pub catalog: Option<PathBuf>,
    #[serde(skip)]
    pub catalog_content: Option<String>,
}

impl LinkConfig {
    pub(crate) fn prepare(&mut self, source: &PathBuf) -> Result<()> {
        if let Some(ref catalog) = self.catalog {
            let catalog_path = source.join(catalog);
            let content = utils::fs::read_string(&catalog_path)
                .map_err(|_| Error::LinkCatalog(catalog_path))?;
            self.catalog_content = Some(content);
        }
        Ok(())
    }
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            allow: None,
            verify: Some(true),
            relative: Some(true),
            catalog: None,
            catalog_content: None,
        }
    }
}
