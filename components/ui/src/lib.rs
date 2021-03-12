#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Psup(#[from] psup_impl::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error + Send>),

    #[error(transparent)]
    Wry(#[from] wry::Error),
}

type Result<T> = std::result::Result<T, Error>;

//mod jsonrpc;
mod services;
mod supervisor;
mod vfs;
mod window;

pub use supervisor::{supervisor, ProcessMessage, SocketFile};
pub use vfs::editor;
pub use window::window;
