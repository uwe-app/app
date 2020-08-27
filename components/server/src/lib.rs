use thiserror::Error;

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
pub mod serve_static;
mod start;

pub use start::*;
