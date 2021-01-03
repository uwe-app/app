pub mod alias;
pub mod build;
pub mod clean;
pub mod dev;
pub mod docs;
pub mod error;
pub mod lang;
pub mod new;
pub mod opts;
pub mod plugin;
pub mod publish;
pub mod server;
pub mod shim;
pub mod sync;
pub mod task;

pub type Result<T> = std::result::Result<T, Error>;
pub type ErrorCallback = fn(Error);

pub use error::Error;
