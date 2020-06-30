use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}
