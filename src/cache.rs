use std::fs;
use std::path::PathBuf;

use crate::git;
use crate::preference::{self, Preferences};
use crate::Error;
use home;

static ROOT_DIR: &str = ".hypertext";
static BIN: &str = "bin";
static ENV: &str = "env";

static BLUEPRINT_NAME: &str = "blueprint";

static STANDALONE_REPO: &str = "https://github.com/hypertext-live/standalone";
static STANDALONE_NAME: &str = "standalone";

static DOCUMENTATION_REPO: &str = "https://github.com/hypertext-live/documentation";
static DOCUMENTATION_NAME: &str = "documentation";

static RELEASE_NAME: &str = "release";

pub enum CacheComponent {
    Blueprint,
    Standalone,
    Documentation,
    Release,
}

pub fn get_root_dir() -> Result<PathBuf, Error> {
    let cache = home::home_dir();
    if let Some(ref cache) = cache {
        let mut buf = cache.clone();
        buf.push(ROOT_DIR);
        if !buf.exists() {
            fs::create_dir(&buf)?;
        }
        return Ok(buf);
    }
    Err(Error::new(format!("Could not determine home directory")))
}

pub fn get_env_file() -> Result<PathBuf, Error> {
    let mut env = get_root_dir()?;
    env.push(ENV);
    Ok(env)
}

pub fn get_bin_dir() -> Result<PathBuf, Error> {
    let mut bin = get_root_dir()?;
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

pub fn get_blueprint_dir() -> Result<PathBuf, Error> {
    let mut buf = get_root_dir()?;
    buf.push(BLUEPRINT_NAME);
    Ok(buf)
}

pub fn get_standalone_url() -> String {
    STANDALONE_REPO.to_string()
}

pub fn get_standalone_dir() -> Result<PathBuf, Error> {
    let mut buf = get_root_dir()?;
    buf.push(STANDALONE_NAME);
    Ok(buf)
}

pub fn get_docs_url() -> String {
    DOCUMENTATION_REPO.to_string()
}

pub fn get_docs_dir() -> Result<PathBuf, Error> {
    let mut buf = get_root_dir()?;
    buf.push(DOCUMENTATION_NAME);
    Ok(buf)
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

pub fn get_release_dir() -> Result<PathBuf, Error> {
    let mut buf = get_root_dir()?;
    buf.push(RELEASE_NAME);
    Ok(buf)
}

pub fn get_release_bin_dir() -> Result<PathBuf, Error> {
    let mut buf = get_release_dir()?;
    buf.push(BIN);
    Ok(buf)
}

pub fn update(prefs: &Preferences, components: Vec<CacheComponent>) -> Result<(), Error> {
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
        }
    }
    Ok(())
}
