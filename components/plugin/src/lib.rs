mod archive;
mod compute;
mod error;
mod installer;
mod linter;
mod packager;
mod reader;
mod registry;
mod resolver;
mod uploader;

type Result<T> = std::result::Result<T, error::Error>;
pub type Registry<'r> = Box<dyn registry::RegistryAccess + Send + Sync + 'r>;

pub use error::Error;
pub use linter::lint;
pub use packager::pack;
pub use resolver::resolve;
pub use uploader::publish;
