mod archive;
mod compute;
mod dependencies;
mod download;
mod error;
mod install;
mod installer;
mod linter;
mod list;
mod packager;
mod publisher;
mod reader;
mod registry;
mod system_plugins;

type Result<T> = std::result::Result<T, error::Error>;
pub type Registry<'r> = Box<dyn registry::RegistryAccess + Send + Sync + 'r>;

pub use self::publisher::publish;
pub use download::get;
pub use error::Error;
pub use install::install;
pub use installer::{
    dependency_installed, install_dependency, install_path, install_repo,
    installation_dir, peek, version_installed,
};
pub use linter::lint;
pub use list::list_dependencies;
pub use packager::pack;
pub use registry::{
    check_for_updates, new_registry, update_registry, RegistryAccess,
    RegistryFileAccess,
};
pub use system_plugins::install_docs;
