use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;

use config::Page;
use config::indexer::QueryList;

use super::Error;

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
    // Assets and other files that should be copied
    pub other: HashMap<Arc<PathBuf>, PathBuf>,

    // Store queries for expansion later
    pub queries: Vec<(QueryList, Arc<PathBuf>)>,

    // Custom page specific layouts
    pub layouts: HashMap<Arc<PathBuf>, PathBuf>,
    // The default layout
    pub layout: Option<Arc<PathBuf>>,

    pub partials: Vec<Arc<PathBuf>>,
    pub includes: Vec<Arc<PathBuf>>,
    pub resources: Vec<Arc<PathBuf>>,
    pub locales: Vec<Arc<PathBuf>>,
    pub data_sources: Vec<Arc<PathBuf>>,
    pub short_codes: Vec<Arc<PathBuf>>,

    // TODO: books too!

    pub links: LinkMap,
}

#[derive(Debug, Default)]
pub struct LinkMap {
    pub sources: HashMap<Arc<PathBuf>, Arc<String>>,
    pub reverse: HashMap<Arc<String>, Arc<PathBuf>>
}
