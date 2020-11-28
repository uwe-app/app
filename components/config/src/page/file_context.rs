use std::io;
use std::path::PathBuf;

use chrono::prelude::*;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::utils::href::UrlPath;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileContext {
    pub name: Option<String>,
    pub path: UrlPath,
    pub source: PathBuf,
    pub template: PathBuf,
    pub modified: DateTime<Utc>,
    #[serde(skip)]
    pub target: PathBuf,
}

impl FileContext {
    pub fn new(
        base: &PathBuf,
        source: PathBuf,
        target: PathBuf,
        template: PathBuf,
    ) -> Self {
        let name = if let Some(stem) = &source.file_stem() {
            Some(stem.to_string_lossy().into_owned())
        } else {
            None
        };

        let rel = match source.strip_prefix(base) {
            Ok(rel) => rel,
            Err(_) => source.as_path(),
        };

        let path = UrlPath::from(rel);

        Self {
            name,
            path,
            source,
            target,
            template,
            modified: Utc::now(),
        }
    }

    pub fn resolve_metadata(&mut self) -> io::Result<()> {
        if let Ok(ref metadata) = self.source.metadata() {
            if let Ok(modified) = metadata.modified() {
                self.modified = DateTime::from(modified);
            }
        }
        Ok(())
    }
}
