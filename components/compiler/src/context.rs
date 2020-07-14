use std::sync::{Arc, RwLock};
use once_cell::sync::OnceCell;

use config::{Config, RuntimeOptions};
use locale::Locales;
use datasource::DataSourceMap;

#[derive(Debug, Default)]
pub struct BuildContext {
    pub config: Config,
    pub options: RuntimeOptions,
    pub datasource: DataSourceMap,
    pub locales: Locales,
}

impl BuildContext {
    pub fn new(
        config: Config,
        options: RuntimeOptions,
        datasource: DataSourceMap,
        locales: Locales) -> Self {
        Self { config, options, datasource, locales }
    }
}

pub fn livereload() -> &'static Arc<RwLock<Option<String>>> {
    static INSTANCE: OnceCell<Arc<RwLock<Option<String>>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        Arc::new(RwLock::new(None))
    })
}
