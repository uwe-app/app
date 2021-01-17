mod error;
mod web_host;

type Result<T> = std::result::Result<T, error::Error>;

pub use error::Error;
