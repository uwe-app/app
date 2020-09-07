use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use collator::{self, Collation};
use config::{Config, RuntimeOptions};

#[derive(Debug, Default)]
pub struct BuildContext {
    pub config: Arc<Config>,
    pub options: Arc<RuntimeOptions>,
    pub collation: Arc<Collation>,
}

impl BuildContext {
    pub fn strip_locale(&self, file: &PathBuf) -> PathBuf {
        let languages = self.options.locales.get_translations();
        if let Some((_lang, path)) =
            collator::get_locale_file_info(&file.as_path(), &languages)
        {
            return path;
        }
        file.to_path_buf()
    }
}

pub fn livereload() -> &'static Arc<RwLock<Option<String>>> {
    static INSTANCE: OnceCell<Arc<RwLock<Option<String>>>> = OnceCell::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(None)))
}
