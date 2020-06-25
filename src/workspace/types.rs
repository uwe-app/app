use crate::config::Config;

pub struct Workspace {
    pub config: Config,
}

impl Workspace {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}
