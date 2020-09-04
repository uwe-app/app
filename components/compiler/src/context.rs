use std::path::PathBuf;
use once_cell::sync::OnceCell;
use std::sync::{Arc, RwLock};

use collator::CollateInfo;
use config::{Config, RuntimeOptions};
use locale::LocaleName;

#[derive(Debug, Default)]
pub struct BuildContext {
    pub config: Config,
    pub options: RuntimeOptions,
    pub collation: CollateInfo,
}

impl BuildContext {
    pub fn new(
        config: Config,
        options: RuntimeOptions,
        collation: CollateInfo,
    ) -> Self {
        Self {
            config,
            options,
            collation,
        }
    }
}

#[derive(Debug)]
pub struct CompileTarget {
    pub lang: LocaleName,
    pub path: PathBuf,
}

pub fn livereload() -> &'static Arc<RwLock<Option<String>>> {
    static INSTANCE: OnceCell<Arc<RwLock<Option<String>>>> = OnceCell::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(None)))
}
