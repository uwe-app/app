use std::sync::{Arc, RwLock};
use once_cell::sync::OnceCell;

use config::{Config, RuntimeOptions};

use datasource::DataSourceMap;
//use locale::Locales;

#[derive(Default)]
pub struct Runtime {
    pub config: Config,
    pub options: RuntimeOptions,
    pub datasource: DataSourceMap,
    //pub locales: Locales,
    //pub livereload: Option<String>,
}

pub fn runtime() -> &'static Arc<RwLock<Runtime>> {
    static INSTANCE: OnceCell<Arc<RwLock<Runtime>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let r = Runtime { ..Default::default() };
        Arc::new(RwLock::new(r))
    })
}
