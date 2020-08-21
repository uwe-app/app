use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use config::indexer::QueryList;
use config::Page;

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

    // Locale specific pages
    pub locale_pages: HashMap<String, HashMap<Arc<PathBuf>, Page>>,

    // Pages that have permalinks map the 
    // permalink to the computed href so that
    // we can configure redirects for permalinks.
    pub permalinks: HashMap<String, String>,

    // Assets and other files that should be copied
    pub other: HashMap<Arc<PathBuf>, PathBuf>,

    // Everything we need to build in pages and other
    // with the target output files
    pub targets: HashMap<Arc<PathBuf>, PathBuf>,

    // Pages located for feed configurations.
    //
    // The hash map key is the key for the feed congfiguration 
    // and each entry is a page path that can be used to 
    // locate the page data in `pages`.
    pub feeds: HashMap<String, Vec<Arc<PathBuf>>>,

    // Store queries for expansion later
    pub queries: Vec<(QueryList, Arc<PathBuf>)>,

    // List of series
    pub series: HashMap<String, Vec<Arc<PathBuf>>>,

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
    pub fn remove_page(&mut self, p: &PathBuf) -> Page {
        self.targets.remove(p);
        self.pages.remove(p).unwrap()
    }

    // Rewrite destination paths.
    //
    // Used for multi-lingual output to locale specific folders.
    pub fn rewrite(&mut self, lang: &str, from: &PathBuf, to: &PathBuf) -> Result<()> {
        for (_path, page) in self.pages.iter_mut() {
            page.set_language(lang);
            page.rewrite_target(&from, &to)?;
        }

        let mut tmp: HashMap<Arc<PathBuf>, PathBuf> = HashMap::new();
        for (k, target) in self.targets.drain() {
            let new_target = to.join(target.strip_prefix(&from)?);
            tmp.entry(k).or_insert(new_target);
        }

        self.targets = tmp;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct LinkMap {
    pub sources: HashMap<Arc<PathBuf>, Arc<String>>,
    pub reverse: HashMap<Arc<String>, Arc<PathBuf>>,
}
