#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    ClientError(String),

    #[error(transparent)]
    Wry(#[from] wry::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod jsonrpc;
mod webview_ipc;
mod window;

pub use window::window;
