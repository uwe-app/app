use std::path::PathBuf;
use std::sync::Arc;

use collator::{self, Collation};
use config::{Config, RuntimeOptions};
use locale::Locales;

use crate::ParseData;

#[derive(Debug, Default)]
pub struct CompilerOutput {
    pub data: Vec<ParseData>,
    // Files that were processed so the renderer
    // can update the manifest
    pub files: Vec<Arc<PathBuf>>,
}

#[derive(Debug, Default)]
pub struct BuildContext {
    pub config: Arc<Config>,
    pub options: Arc<RuntimeOptions>,
    pub collation: Arc<Collation>,
    pub locales: Arc<Locales>,
}
