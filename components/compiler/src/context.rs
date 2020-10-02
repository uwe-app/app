use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use collator::{self, Collation};
use config::{plugin_cache::PluginCache, Config, RuntimeOptions};
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
    pub plugins: Option<Arc<PluginCache>>,
    pub locales: Arc<Locales>,
    pub collation: Arc<RwLock<Collation>>,
}
