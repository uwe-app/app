use config::Config;

mod compile;
mod finder;
pub mod project;

#[derive(Clone)]
pub struct Workspace {
    pub config: Config,
}

impl Workspace {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

pub use finder::find;
pub use compile::compile;
pub use compile::compile_from;
pub use compile::compile_project;
