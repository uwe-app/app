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

    #[error("No menu {0} resolving to file {1}")]
    NoMenuFile(String, PathBuf),

    #[error("No site configuration in {0}")]
    NoSiteConfig(PathBuf),

    #[error("No author found for {0}")]
    NoAuthor(String),

    #[error("Hook defined in {0} has an empty command path")]
    HookPathEmpty(PathBuf),

    #[error("Hook requires the file {0}")]
    NoHookFile(PathBuf),

    #[error(
        "Dependency {0} wants the feature {1} but the feature is not available"
    )]
    NoFeature(String, String),

    #[error("Dependency {0} wants to apply layouts but the plugin has no templates for the engine {1}")]
    ApplyLayoutNoTemplateForEngine(String, String),

    #[error("Dependency {0} wants to apply layouts but the plugin has no layouts for the engine {1}")]
    ApplyLayoutNoLayouts(String, String),

    #[error("Dependency {0} wants to apply the layout {2} but the plugin does not have the layout: {2} (engine: {1})")]
    ApplyLayoutNoLayoutForKey(String, String, String),

    //#[error("Page {0} is outside the source directory {1}")]
    //PageOutsideSource(PathBuf, PathBuf),
    #[error("Failed to read link catalog {0}")]
    LinkCatalog(PathBuf),

    #[error("Too many redirects, limit is {0}")]
    TooManyRedirects(usize),

    #[error("Cyclic redirect {stack} <-> {key}")]
    CyclicRedirect { stack: String, key: String },

    #[error("Cyclic feature {0}")]
    CyclicFeature(String),

    #[error("Feature stack depth has exceeded the maximum {0}")]
    FeatureStackTooLarge(usize),

    #[error("Redirect file {0} already exists")]
    RedirectFileExists(PathBuf),

    #[error("Template engine {0} is not supported")]
    UnsupportedTemplateEngine(String),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    InvalidUri(#[from] http::uri::InvalidUri),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Lang(#[from] unic_langid::LanguageIdentifierError),
}

type Result<T> = std::result::Result<T, Error>;

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

mod config;
pub mod date;
pub mod engine;
pub mod feed;
mod fluent;
pub mod generator;
pub mod hook;
pub mod indexer;
mod layout;
pub mod license;
mod link;
mod live_reload;
mod menu;
mod minify;
mod options;
pub mod page;
pub mod plugin;
pub mod plugin_cache;
pub mod profile;
pub mod redirect;
pub mod robots;
pub mod script;
pub mod search;
pub mod server;
pub mod sitemap;
pub mod style;
pub mod syntax;
pub mod transform;

pub(crate) mod utils;

pub use self::utils::{href, markdown};
pub use config::*;
//pub use engine::TemplateEngine;
pub use fluent::{FluentConfig, CORE_FTL};
pub use hook::HookConfig;
pub use indexer::{IndexQuery, KeyType, QueryResult};
pub use menu::{MenuEntry, MenuReference, MenuResult, MENU};
pub use options::{DestinationBuilder, FileType, LinkOptions, RuntimeOptions};
pub use page::{Author, Page, PageLink, PaginateInfo};
pub use plugin::*;
pub use profile::{ProfileName, ProfileSettings, RenderTypes};
pub use redirect::*;
pub use search::SearchConfig;

pub use semver;
