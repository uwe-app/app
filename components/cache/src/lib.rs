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

static RUNTIME_REPO: &str =
    "https://github.com/hypertext-live/runtime";
static RUNTIME_NAME: &str = "runtime";

static DOCUMENTATION_NAME: &str = "documentation/docs";
static SYNTAX_NAME: &str = "syntax";

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
    Runtime,
    Blueprint,
    Release,
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

pub fn get_runtime_url() -> String {
    RUNTIME_REPO.to_string()
}

pub fn get_runtime_dir() -> io::Result<PathBuf> {
    Ok(dirs::get_root_dir()?.join(RUNTIME_NAME))
}

pub fn get_docs_dir() -> io::Result<PathBuf> {
    Ok(get_runtime_dir()?.join(DOCUMENTATION_NAME))
}

pub fn get_syntax_dir() -> io::Result<PathBuf> {
    Ok(get_runtime_dir()?.join(SYNTAX_NAME))
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
            CacheComponent::Runtime => {
                let url = get_runtime_url();
                let dir = get_runtime_dir()?;
                git::clone_or_fetch(&url, &dir, true)?;
            }
            CacheComponent::Blueprint => {
                let url = get_blueprint_url(prefs);
                let dir = get_blueprint_dir()?;
                git::clone_or_fetch(&url, &dir, true)?;
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
