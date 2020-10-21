pub mod alias;
pub mod build;
pub mod docs;
pub mod error;
pub mod list;
pub mod new;
pub mod opts;
pub mod plugin;
pub mod publish;
pub mod run;

pub type Result<T> = std::result::Result<T, Error>;
pub type ErrorCallback = fn(Error);

pub use error::Error;
