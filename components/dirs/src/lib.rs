use std::path::PathBuf;
use std::{fs, io};

static ROOT_DIR: &str = ".uwe";

static BIN: &str = "bin";
static ENV: &str = "env";

static CACHE_NAME: &str = "cache";
static SRC_NAME: &str = "src";
static BLUEPRINT_NAME: &str = "blueprint";

static SITES_NAME: &str = "sites";
static SITES_FILE: &str = "sites.toml";

static RUNTIME_REPO: &str = "https://github.com/uwe-app/runtime";

static RUNTIME_NAME: &str = "runtime";
static REGISTRY_NAME: &str = "registry";

static DOCUMENTATION_NAME: &str = "documentation/docs";
static SYNTAX_NAME: &str = "syntax";
static RELEASE_NAME: &str = "release";

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

pub fn runtime_url() -> String {
    RUNTIME_REPO.to_string()
}

pub fn runtime_dir() -> io::Result<PathBuf> {
    Ok(root_dir()?.join(RUNTIME_NAME))
}

pub fn cache_dir() -> io::Result<PathBuf> {
    let cache = root_dir()?.join(CACHE_NAME);
    if !cache.exists() {
        fs::create_dir(&cache)?;
    }
    Ok(cache)
}

pub fn cache_src_dir() -> io::Result<PathBuf> {
    let src = cache_dir()?.join(SRC_NAME);
    if !src.exists() {
        fs::create_dir(&src)?;
    }
    Ok(src)
}

pub fn registry_dir() -> io::Result<PathBuf> {
    Ok(runtime_dir()?.join(REGISTRY_NAME))
}

pub fn docs_dir() -> io::Result<PathBuf> {
    Ok(runtime_dir()?.join(DOCUMENTATION_NAME))
}

pub fn syntax_dir() -> io::Result<PathBuf> {
    Ok(runtime_dir()?.join(SYNTAX_NAME))
}

pub fn blueprint_dir() -> io::Result<PathBuf> {
    Ok(runtime_dir()?.join(BLUEPRINT_NAME))
}

pub fn release_dir() -> io::Result<PathBuf> {
    let mut buf = root_dir()?;
    buf.push(RELEASE_NAME);
    Ok(buf)
}

pub fn release_bin_dir() -> io::Result<PathBuf> {
    let mut buf = release_dir()?;
    buf.push(BIN);
    Ok(buf)
}

pub use home::home_dir;
