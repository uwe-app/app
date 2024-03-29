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
    CollectionDocumentNotAnObject,

    #[error("Data source document must have an id")]
    CollectionDocumentNoId,

    #[error("Page size {0} is not large enough, must be greater than one")]
    PageSizeTooSmall(usize),

    #[error("No data source with name {0}")]
    NoCollection(String),

    #[error("No index with name {0}")]
    NoIndex(String),

    //#[error("No feed template file {0}")]
    //NoFeedTemplate(PathBuf),
    #[error("No configuration {conf} for collection {key}")]
    NoCollectionConf { conf: String, key: String },

    #[error("Target {docs} for collection {key} does not exist")]
    NoCollectionDocuments { docs: PathBuf, key: String },

    #[error("Path for 'document' provider must be a file ({0})")]
    NotDocumentFile(PathBuf),

    #[error("Unsupported type for 'document' provider in {0} (must be an object or array)")]
    UnsupportedType(PathBuf),

    #[error("XML document {0} is empty")]
    EmptyXmlDocument(PathBuf),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Csv(#[from] csv::Error),

    #[error(transparent)]
    Xml(#[from] xml::reader::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Collator(#[from] collator::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub mod identifier;
mod indexer;
pub mod provider;
pub mod synthetic;
mod xml_value;

pub use indexer::*;
