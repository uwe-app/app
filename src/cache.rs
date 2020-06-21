use std::fs;
use std::path::PathBuf;

use home;
use crate::Error;
use crate::git;
use crate::preference::{self, Preferences};

static ROOT_DIR: &str = ".hypertext";

static BLUEPRINT_NAME: &str = "blueprint";

static STANDALONE_REPO: &str = "https://github.com/hypertext-live/standalone";
static STANDALONE_NAME: &str = "standalone";

static DOCUMENTATION_REPO: &str = "https://github.com/hypertext-live/documentation";
static DOCUMENTATION_NAME: &str = "documentation";

pub enum CacheComponent {
    Blueprint,
    Standalone,
    Documentation,
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
    Err(
        Error::new(
            format!("Could not determine home directory")))
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

pub fn update(prefs: &Preferences, components: Vec<CacheComponent>) -> Result<(), Error> {
    for c in components {
        match c {
            CacheComponent::Blueprint => {
                let url = get_blueprint_url(prefs);
                let dir = get_blueprint_dir()?;
                git::clone_or_fetch(&url, &dir, true)?;
            },
            CacheComponent::Standalone => {
                let url = get_standalone_url();
                let dir = get_standalone_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            },
            CacheComponent::Documentation => {
                let url = get_docs_url();
                let dir = get_docs_dir()?;
                git::clone_or_fetch(&url, &dir, false)?;
            },
        }
    }
    Ok(())
}
