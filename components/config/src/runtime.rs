use std::sync::{Arc, RwLock};
use once_cell::sync::OnceCell;

use super::{Config, RuntimeOptions};

pub struct Runtime {
    pub config: Config,
    pub options: RuntimeOptions,
}

pub fn runtime() -> &'static Arc<RwLock<Runtime>> {
    static INSTANCE: OnceCell<Arc<RwLock<Runtime>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let r = Runtime {config: Default::default(), options: Default::default()};
        Arc::new(RwLock::new(r))
    })
}
