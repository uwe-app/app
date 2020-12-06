pub mod alias;
pub mod build;
pub mod clean;
pub mod docs;
pub mod error;
pub mod lang;
pub mod list;
pub mod new;
pub mod opts;
pub mod plugin;
pub mod publish;
pub mod server;

pub type Result<T> = std::result::Result<T, Error>;
pub type ErrorCallback = fn(Error);

pub use error::Error;
