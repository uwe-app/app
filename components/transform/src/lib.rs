use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // NOTE: Cannot pass the RewritingError transparently as it is 
    // NOTE: not safe to Send via threads.
    #[error("{0}")]
    Rewriting(String),

    #[error(transparent)]
    Toc(#[from] toc::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub mod html;
