use std::path::PathBuf;
use std::{fs, io};

static ROOT_DIR: &str = ".uwe";

static BIN: &str = "bin";
static ENV: &str = "env";

static SITES_NAME: &str = "sites";
static SITES_FILE: &str = "sites.toml";

static RELEASES_REPO: &str = "https://github.com/uwe-app/releases";
static REGISTRY_REPO: &str = "https://github.com/uwe-app/registry";

static REGISTRY: &str = "registry";
static RELEASES: &str = "releases";
static DOWNLOADS: &str = "downloads";

pub fn root_dir() -> io::Result<PathBuf> {
    let cache = home::home_dir();
    if let Some(ref cache) = cache {
        let mut buf = cache.clone();
        buf.push(ROOT_DIR);
        if !buf.exists() {
            std::fs::create_dir(&buf)?;
        }
        return Ok(buf);
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not determine home directory".to_string(),
    ))
}

pub fn sites_dir() -> io::Result<PathBuf> {
    let mut bin = root_dir()?;
    bin.push(SITES_NAME);
    if !bin.exists() {
        fs::create_dir(&bin)?;
    }
    Ok(bin)
}

pub fn sites_manifest() -> io::Result<PathBuf> {
    Ok(root_dir()?.join(SITES_FILE))
}

pub fn env_file() -> io::Result<PathBuf> {
    let mut env = root_dir()?;
    env.push(ENV);
    Ok(env)
}

pub fn bin_dir() -> io::Result<PathBuf> {
    let mut bin = root_dir()?;
    bin.push(BIN);
    if !bin.exists() {
        fs::create_dir(&bin)?;
    }
    Ok(bin)
}

pub fn releases_url() -> String {
    RELEASES_REPO.to_string()
}

pub fn registry_url() -> String {
    REGISTRY_REPO.to_string()
}

pub fn releases_dir() -> io::Result<PathBuf> {
    Ok(root_dir()?.join(RELEASES))
}

pub fn registry_dir() -> io::Result<PathBuf> {
    Ok(root_dir()?.join(REGISTRY))
}

pub fn downloads_dir() -> io::Result<PathBuf> {
    Ok(registry_dir()?.join(DOWNLOADS))
}

pub use home::home_dir;
