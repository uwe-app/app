use std::path::PathBuf;
use once_cell::sync::OnceCell;
use std::sync::{Arc, RwLock};

use collator::CollateInfo;
use config::{Config, RuntimeOptions, LocaleName};

#[derive(Debug, Default)]
pub struct CompileTarget {
    pub lang: LocaleName,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct BuildContext {
    pub config: Config,
    pub options: RuntimeOptions,
    pub collation: CollateInfo,
}

#[derive(Debug, Default)]
pub struct CompileInfo {
    pub target: Arc<CompileTarget>,
    pub context: Arc<BuildContext>,
    pub sources: Arc<Vec<PathBuf>>,
}

pub fn livereload() -> &'static Arc<RwLock<Option<String>>> {
    static INSTANCE: OnceCell<Arc<RwLock<Option<String>>>> = OnceCell::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(None)))
}
