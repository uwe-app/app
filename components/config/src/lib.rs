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

    /*
    #[error("No menu {0} resolving to file {1}")]
    NoMenuFile(String, PathBuf),
    */
    #[error("No site settings in {0}, project requires a site.toml file")]
    NoSiteConfig(PathBuf),

    #[error("Hook defined in {0} has an empty command path")]
    HookPathEmpty(PathBuf),

    #[error("Hook requires the file {0}")]
    NoHookFile(PathBuf),

    #[error("Collections `from` path {0} may not be absolute")]
    FromAbsolute(PathBuf),

    #[error("Failed to parse git URL: {0} ({1})")]
    GitUrlParseFail(String, String),

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

    #[error("Filters given for {0} dependency but no plugin files matched")]
    ApplyFiltersNoMatch(String),

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

    #[error("The value {0} for 'rel' is not supported")]
    InvalidRelValue(String),

    #[error("Plugin ref spec {0} is not valid (namespace required)")]
    InvalidPluginSpecName(String),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

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

    #[error(transparent)]
    ReqParse(#[from] semver::ReqParseError),
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
pub mod license;
mod link;
mod live_reload;
pub mod memfs;
mod menu;
mod minify;
mod options;
pub mod page;
pub mod plugin;
pub mod plugin_cache;
pub mod profile;
pub mod redirect;
pub mod repository;
pub mod robots;
pub mod script;
pub mod search;
pub mod server;
pub mod sitemap;
pub mod style;
pub mod sync;
pub mod syntax;
pub mod tags;
pub mod test;
pub mod transform;

pub(crate) mod utils;

pub use self::utils::{href, markdown};
pub use config::*;
pub use fluent::{FluentConfig, CORE_FTL};
pub use hook::HookConfig;
pub use indexer::{IndexQuery, KeyType, QueryResult, SourceProvider};
pub use menu::{MenuEntry, MenuReference, MenuResult};
pub use options::{DestinationBuilder, FileType, LinkOptions, RuntimeOptions};
pub use page::{Author, Page, PageLink, PaginateInfo};
pub use plugin::*;
pub use profile::{ProfileName, ProfileSettings, RenderTypes};
pub use redirect::*;
pub use search::SearchConfig;

pub use semver;

/// Get the release directory for the current executable version.
///
/// Only safe to be called after `opts::project_path()` so that
/// the executable version is correct.
pub fn current_release_dir() -> std::io::Result<PathBuf> {
    let version = generator::version();
    Ok(dirs::releases_dir()?.join(version))
}

/// Get the plugins directory relative to the release directory
/// for the current executable version.
pub fn plugins_dir() -> std::io::Result<PathBuf> {
    let dir = current_release_dir()?.join(crate::PLUGINS);
    if !dir.exists() {
        // Must use create_dir_all() for the case when we are
        // testing locally and have bumped the version number
        // and therefore do not yet have an installation directory
        // for the release.
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}
