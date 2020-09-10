use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to resolve project directory for {0}")]
    ProjectResolve(PathBuf),

    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("No socket address for {0}")]
    NoSocketAddress(String),

    #[error("Missing book configuration {0}")]
    NoBookConfig(PathBuf),

    #[error("No site configuration in {0}")]
    NoSiteConfig(PathBuf),

    #[error("No author found for {0}")]
    NoAuthor(String),

    #[error("Page {0} is outside the source directory {1}")]
    PageOutsideSource(PathBuf, PathBuf),

    #[error("Failed to read link catalog {0}")]
    LinkCatalog(PathBuf),

    #[error("Too many redirects, limit is {0}")]
    TooManyRedirects(usize),

    #[error("Cyclic redirect: {stack} <-> {key}")]
    CyclicRedirect { stack: String, key: String },

    #[error("Redirect file {0} already exists")]
    RedirectFileExists(PathBuf),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    InvalidUri(#[from] http::uri::InvalidUri),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Lang(#[from] unic_langid::LanguageIdentifierError),
}

type Result<T> = std::result::Result<T, Error>;

pub mod app;
pub mod book;
mod config;
pub mod feed;
mod file;
pub mod indexer;
pub mod link;
mod matcher;
mod page;
pub mod path;
mod profile;
pub mod redirect;
pub mod robots;
pub mod script;
pub mod search;
pub mod server;
pub mod sitemap;
pub mod style;
pub mod syntax;
pub mod transform;

pub fn get_short_codes_location() -> Result<PathBuf> {
    //self.source.join(config::SHORT_CODES.to_string())
    Ok(dirs::get_root_dir()?.join("shortcodes/site/partials"))
}

pub fn to_url_string(
    scheme: &str,
    host: &str,
    port: impl Into<Option<u16>>,
) -> String {
    let url = if let Some(port) = port.into() {
        if port == 80 || port == 443 {
            format!("{}{}{}", scheme, crate::config::SCHEME_DELIMITER, host)
        } else {
            format!(
                "{}{}{}:{}",
                scheme,
                crate::config::SCHEME_DELIMITER,
                host,
                port
            )
        }
    } else {
        format!("{}{}{}", scheme, crate::config::SCHEME_DELIMITER, host)
    };
    url
}

pub use crate::config::*;
pub use file::{FileInfo, FileOptions, FileType};
pub use indexer::{IndexQuery, KeyType, QueryResult};
pub use page::{Author, CollatedPage, Page, PageLink, PaginateInfo};
pub use profile::{ProfileName, ProfileSettings, RenderTypes, RuntimeOptions};
pub use redirect::*;
pub use search::{SEARCH_JS, SEARCH_WASM};
