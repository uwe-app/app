#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Wry(#[from] wry::Error),
}

type Result<T> = std::result::Result<T, Error>;

mod webview_ipc;
mod window;

pub use window::window;
