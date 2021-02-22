#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error + Send>),

    #[error(transparent)]
    Wry(#[from] wry::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod jsonrpc;
mod services;
mod window;

pub use window::window;
