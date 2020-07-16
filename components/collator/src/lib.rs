use thiserror::Error;

pub mod collation;
pub mod collator;

pub use collation::*;
pub use collator::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Poison(#[from] std::sync::PoisonError<CollateInfo>),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Loader(#[from] loader::Error),
}

type Result<T> = std::result::Result<T, Error>;

