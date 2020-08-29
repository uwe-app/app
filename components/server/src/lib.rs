use once_cell::sync::OnceCell;
use thiserror::Error;
use config::server::ServerConfig;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Warp(#[from] warp::Error),

    #[error(transparent)]
    TrySend(#[from] tokio::sync::mpsc::error::TrySendError<(bool, std::net::SocketAddr)>),
}

type Result<T> = std::result::Result<T, Error>;

pub mod redirect;
mod router;
mod bind;

pub use bind::*;

pub fn configure(config: ServerConfig) -> &'static ServerConfig {
    static INSTANCE: OnceCell<ServerConfig> = OnceCell::new();
    INSTANCE.get_or_init(|| config)
}
