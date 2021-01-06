mod archive;
mod compute;
mod error;
mod installer;
mod linter;
mod packager;
mod reader;
mod registry;
mod resolver;
mod system_plugins;
mod uploader;

type Result<T> = std::result::Result<T, error::Error>;
pub type Registry<'r> = Box<dyn registry::RegistryAccess + Send + Sync + 'r>;

pub use error::Error;
pub use installer::install_registry;
pub use linter::lint;
pub use packager::pack;
pub use registry::{RegistryFileAccess, RegistryAccess, new_registry};
pub use resolver::resolve;
pub use system_plugins::{install_blueprint, install_docs};
pub use uploader::publish;
