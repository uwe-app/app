use config::server::ServerConfig;
use once_cell::sync::OnceCell;

mod channels;
mod drop_privileges;
mod error;
mod launch;
//mod live_render;
//pub mod redirect;
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

/// When the web server routes are configured various strings need
/// to have the `static` lifetime. This function converts a server
/// configuration to a `&'static` reference so strings in the server
/// configuration can be used when constructing the warp filters.
pub fn configure(config: ServerConfig) -> &'static ServerConfig {
    static INSTANCE: OnceCell<ServerConfig> = OnceCell::new();
    INSTANCE.get_or_init(|| config)
}
