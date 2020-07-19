use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;

use config::Page;

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
    // Unrecognized files that should be copied
    pub other: HashMap<Arc<PathBuf>, PathBuf>,

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

}

