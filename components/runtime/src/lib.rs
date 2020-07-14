use std::sync::{Arc, RwLock};
use once_cell::sync::OnceCell;

use config::{Config, RuntimeOptions};

use datasource::DataSourceMap;
use locale::Locales;

#[derive(Default)]
pub struct Runtime {
    pub config: Config,
    pub options: RuntimeOptions,
    pub datasource: DataSourceMap,
    //pub locales: Locales,
}

// This logic for a static reference to the configuration settings and 
// runtime options exists because we need to use this data in the handlebars 
// helpers. An earlier implementation serialized this data and passed it to 
// the templates which then deserialized and used it which is a very bad idea 
// for several reasons not least of which it is very, very slow.
//
// We do not use a Mutex otherwise nested handlebars templates will not be 
// able to acquire a lock if a parent helper has already acquired a lock. In 
// that situation we get a deadlock and the program hangs. Using a RwLock fixes 
// the issue given the notes on mutability below.
//
// The configuration settings are immutable once Config has loaded 
// the data and set defaults. The runtime options are mutable up until the point 
// when a compilation pass beings then they are considered immutable and callers 
// should only acquire a read lock once the runtime options have been prepared.
pub fn runtime() -> &'static Arc<RwLock<Runtime>> {
    static INSTANCE: OnceCell<Arc<RwLock<Runtime>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let r = Runtime { ..Default::default() };
        Arc::new(RwLock::new(r))
    })
}

pub fn livereload() -> &'static Arc<RwLock<Option<String>>> {
    static INSTANCE: OnceCell<Arc<RwLock<Option<String>>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        Arc::new(RwLock::new(None))
    })
}

pub fn locales() -> &'static Arc<RwLock<Locales>> {
    static INSTANCE: OnceCell<Arc<RwLock<Locales>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        Arc::new(RwLock::new(Default::default()))
    })
}
