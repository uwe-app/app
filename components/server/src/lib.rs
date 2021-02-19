mod channels;
mod drop_privileges;
mod error;
mod launch;
mod reload_server;
mod router;
mod watch;
mod websocket;

pub use channels::*;
pub use error::Error;
pub use launch::*;
pub use watch::watch;

pub type ErrorCallback = fn(Error);
type Result<T> = std::result::Result<T, Error>;
