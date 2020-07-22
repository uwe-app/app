use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;

use config::Page;
use config::indexer::QueryList;

use super::manifest::Manifest;

use super::{Error, Result};

#[derive(Debug, Default)]
pub struct CollateData {
    pub page: Option<Page>,
}

#[derive(Debug, Default)]
pub struct CollateInfo {
    pub errors: Vec<Error>,
    pub all: Vec<Arc<PathBuf>>,
    pub dirs: Vec<Arc<PathBuf>>,
    pub files: Vec<Arc<PathBuf>>,
    pub assets: Vec<Arc<PathBuf>>,

    // Pages to compile
    pub pages: HashMap<Arc<PathBuf>, Page>,

    // FIXME: we should be able to dispense with `other`
    // FIXME: now that we have the `targets` map
    //
    // Assets and other files that should be copied
    pub other: HashMap<Arc<PathBuf>, PathBuf>,

    // Everything we need to build in pages and other
    // with the target output files
    pub targets: HashMap<Arc<PathBuf>, PathBuf>,

    // Store queries for expansion later
    pub queries: Vec<(QueryList, Arc<PathBuf>)>,

    // List of series
    pub series: HashMap<String, Arc<PathBuf>>,

    // Custom page specific layouts
    pub layouts: HashMap<Arc<PathBuf>, PathBuf>,
    // The default layout
    pub layout: Option<Arc<PathBuf>>,

    pub partials: Vec<Arc<PathBuf>>,
    pub includes: Vec<Arc<PathBuf>>,
    pub resources: Vec<Arc<PathBuf>>,
    pub locales: Vec<Arc<PathBuf>>,
    pub data_sources: Vec<Arc<PathBuf>>,

    // TODO: books too!

    pub links: LinkMap,

    // Manifest for incremental builds
    pub manifest: Option<Manifest>,
}

impl CollateInfo {

    // Rewrite destination paths.
    //
    // Used for multi-lingual output to locale specific folders.
    pub fn rewrite(&mut self, lang: &str, from: &PathBuf, to: &PathBuf) -> Result<()> {

        for (_path, page) in self.pages.iter_mut() {
            page.set_language(lang);
            page.rewrite_target(&from, &to)?;
        }
    
        let mut tmp: HashMap<Arc<PathBuf>, PathBuf> = HashMap::new();
        for (k, target) in self.other.drain() {
            let new_target = to.join(target.strip_prefix(&from)?);
            tmp.entry(k).or_insert(new_target);
        }

        self.other = tmp;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct LinkMap {
    pub sources: HashMap<Arc<PathBuf>, Arc<String>>,
    pub reverse: HashMap<Arc<String>, Arc<PathBuf>>
}
