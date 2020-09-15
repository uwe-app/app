use std::fs;
use std::io;
use std::path::PathBuf;

use thiserror::Error;

use dirs;
use git;
use preference::{self, Preferences};

static BIN: &str = "bin";
static ENV: &str = "env";

static BLUEPRINT_NAME: &str = "blueprint";
static WORKSPACE_NAME: &str = "workspace";
static WORKSPACE_FILE: &str = "workspace.toml";

static STANDALONE_REPO: &str = "https://github.com/hypertext-live/standalone";
static STANDALONE_NAME: &str = "standalone";

static DOCUMENTATION_REPO: &str =
    "https://github.com/hypertext-live/documentation";
static DOCUMENTATION_NAME: &str = "documentation";

static SHORT_CODES_REPO: &str = "https://github.com/hypertext-live/shortcodes";
static SHORT_CODES_NAME: &str = "shortcodes";

static SYNTAX_REPO: &str = "https://github.com/hypertext-live/syntax";
static SYNTAX_NAME: &str = "syntax";

static SEARCH_REPO: &str = "https://github.com/hypertext-live/search-runtime";
static SEARCH_NAME: &str = "search-runtime";

static FEED_REPO: &str = "https://github.com/hypertext-live/feed";
static FEED_NAME: &str = "feed";

static BOOK_REPO: &str = "https://github.com/hypertext-live/book";
static BOOK_NAME: &str = "book";

static VERSION_BASE: &str =
    "https://raw.githubusercontent.com/hypertext-live/release-";
static VERSION_FILE: &str = "/master/version.toml";

static RELEASE_NAME: &str = "release";

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub enum CacheComponent {
    Blueprint,
    Standalone,
    Documentation,
    Release,
    ShortCode,
    Syntax,
    Search,
    Feed,
    Book,
}

pub fn get_workspace_dir() -> io::Result<PathBuf> {
    let mut bin = dirs::get_root_dir()?;
    bin.push(WORKSPACE_NAME);
    if !bin.exists() {
        fs::create_dir(&bin)?;
    }
    Ok(bin)
}

pub fn get_workspace_manifest() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(WORKSPACE_FILE))
}

pub fn get_env_file() -> io::Result<PathBuf> {
    let mut env = dirs::get_root_dir()?;
    env.push(ENV);
    Ok(env)
}

pub fn get_bin_dir() -> io::Result<PathBuf> {
    let mut bin = dirs::get_root_dir()?;
    bin.push(BIN);
    if !bin.exists() {
        fs::create_dir(&bin)?;
    }
    Ok(bin)
}

pub fn get_blueprint_url(prefs: &Preferences) -> String {
    if let Some(ref blueprint) = prefs.blueprint {
        if let Some(ref url) = blueprint.url {
            return url.clone();
        }
    }
    return preference::BLUEPRINT_URL.to_string();
}

pub fn get_blueprint_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(BLUEPRINT_NAME))
}

pub fn get_standalone_url() -> String {
    STANDALONE_REPO.to_string()
}

pub fn get_standalone_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(STANDALONE_NAME))
}

pub fn get_docs_url() -> String {
    DOCUMENTATION_REPO.to_string()
}

pub fn get_docs_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(DOCUMENTATION_NAME))
}

pub fn get_short_codes_url() -> String {
    SHORT_CODES_REPO.to_string()
}

pub fn get_short_codes_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(SHORT_CODES_NAME))
}

pub fn get_syntax_url() -> String {
    SYNTAX_REPO.to_string()
}

pub fn get_syntax_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(SYNTAX_NAME))
}

pub fn get_search_url() -> String {
    SEARCH_REPO.to_string()
}

pub fn get_search_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(SEARCH_NAME))
}

pub fn get_feed_url() -> String {
    FEED_REPO.to_string()
}

pub fn get_feed_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(FEED_NAME))
}

pub fn get_book_url() -> String {
    BOOK_REPO.to_string()
}

pub fn get_book_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(BOOK_NAME))
}

#[cfg(target_os = "windows")]
pub fn get_release_version() -> String {
    format!("{}{}{}", VERSION_BASE, "windows", VERSION_FILE)
}

#[cfg(target_os = "macos")]
pub fn get_release_version() -> String {
    format!("{}{}{}", VERSION_BASE, "macos", VERSION_FILE)
}

#[cfg(target_os = "linux")]
pub fn get_release_version() -> String {
    format!("{}{}{}", VERSION_BASE, "linux", VERSION_FILE)
}

#[cfg(target_os = "windows")]
pub fn get_release_url() -> String {
    String::from("https://github.com/hypertext-live/release-windows")
}

#[cfg(target_os = "macos")]
pub fn get_release_url() -> String {
    String::from("https://github.com/hypertext-live/release-macos")
}

#[cfg(target_os = "linux")]
pub fn get_release_url() -> String {
    String::from("https://github.com/hypertext-live/release-linux")
}

pub fn get_release_dir() -> io::Result<PathBuf> {
    let mut buf = dirs::get_root_dir()?;
    buf.push(RELEASE_NAME);
    Ok(buf)
}

pub fn get_release_bin_dir() -> io::Result<PathBuf> {
    let mut buf = get_release_dir()?;
    buf.push(BIN);
    Ok(buf)
}

pub fn update(
    prefs: &Preferences,
    components: Vec<CacheComponent>,
) -> Result<(), Error> {
    for c in components {
        match c {
            CacheComponent::Blueprint => {
                let url = get_blueprint_url(prefs);
                let dir = get_blueprint_dir()?;
                git::clone_or_fetch(&url, &dir, true)?;
            }
            CacheComponent::Standalone => {
                let url = get_standalone_url();
                let dir = get_standalone_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
            CacheComponent::Documentation => {
                let url = get_docs_url();
                let dir = get_docs_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
            CacheComponent::Release => {
                let url = get_release_url();
                let dir = get_release_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
            CacheComponent::ShortCode => {
                let url = get_short_codes_url();
                let dir = get_short_codes_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
            CacheComponent::Syntax => {
                let url = get_syntax_url();
                let dir = get_syntax_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
            CacheComponent::Search => {
                let url = get_search_url();
                let dir = get_search_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
            CacheComponent::Feed => {
                let url = get_feed_url();
                let dir = get_feed_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
            CacheComponent::Book => {
                let url = get_book_url();
                let dir = get_book_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            }
        }
    }
    Ok(())
}
