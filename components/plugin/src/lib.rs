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
pub use installer::{
    dependency_installed, install_archive, install_path, install_registry,
    install_repo, installation_dir, version_installed,
};
pub use linter::lint;
pub use packager::pack;
pub use registry::{new_registry, RegistryAccess, RegistryFileAccess};
pub use resolver::resolve;
pub use system_plugins::{install_blueprint, install_docs};
pub use uploader::publish;
