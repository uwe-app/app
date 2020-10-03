pub mod build;
pub mod docs;
pub mod error;
pub mod init;
pub mod plugin;
pub mod publish;
pub mod run;
pub mod site;
pub mod upgrade;

pub type Result<T> = std::result::Result<T, Error>;
pub type ErrorCallback = fn(Error);

pub use error::Error;
