use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Query should be array or object")]
    QueryType,

    #[error("Duplicate document id {key} ({path})")]
    DuplicateId { key: String, path: PathBuf },

    #[error("Type error building index, keys must be string values")]
    IndexKeyType,

    #[error("Data source document should be an object")]
    DataSourceDocumentNotAnObject,

    #[error("Data source document must have an id")]
    DataSourceDocumentNoId,

    #[error("Page size {0} is not large enough, must be greater than one")]
    PageSizeTooSmall(usize),

    #[error("No data source with name {0}")]
    NoDataSource(String),

    #[error("No index with name {0}")]
    NoIndex(String),

    #[error("No feed template file {0}")]
    NoFeedTemplate(PathBuf),

    #[error("No book theme directory {0}")]
    NoBookThemeDirectory(PathBuf),

    #[error("No configuration {conf} for data source {key}")]
    NoDataSourceConf { conf: String, key: String },

    #[error("No {docs} directory for data source {key}")]
    NoDataSourceDocuments { docs: PathBuf, key: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Collator(#[from] collator::Error),
    #[error(transparent)]
    Provider(#[from] provider::DeserializeError),
}

type Result<T> = std::result::Result<T, Error>;

pub mod identifier;
mod indexer;
pub mod provider;
pub mod synthetic;

pub use indexer::*;
