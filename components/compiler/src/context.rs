use once_cell::sync::OnceCell;
use std::sync::{Arc, RwLock};

use collator::{self, Collation};
use config::{Config, RuntimeOptions};
use locale::Locales;

use crate::ParseData;

#[derive(Debug, Default)]
pub struct CompilerOutput {
    pub data: Vec<ParseData>,
}

#[derive(Debug, Default)]
pub struct BuildContext {
    pub config: Arc<Config>,
    pub options: Arc<RuntimeOptions>,
    pub collation: Arc<Collation>,
    pub locales: Arc<Locales>,
}

pub fn livereload() -> &'static Arc<RwLock<Option<String>>> {
    static INSTANCE: OnceCell<Arc<RwLock<Option<String>>>> = OnceCell::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(None)))
}
