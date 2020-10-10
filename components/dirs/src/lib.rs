use std::{fs, io};
use std::path::PathBuf;

static ROOT_DIR: &str = ".uwe";

static BIN: &str = "bin";
static ENV: &str = "env";

static CACHE_NAME: &str = "cache";
static SRC_NAME: &str = "src";
static BLUEPRINT_NAME: &str = "blueprint";

static WORKSPACE_NAME: &str = "sites";
static WORKSPACE_FILE: &str = "sites.toml";

static RUNTIME_REPO: &str = "https://github.com/uwe-app/runtime";

static RUNTIME_NAME: &str = "runtime";
static REGISTRY_NAME: &str = "registry";

static DOCUMENTATION_NAME: &str = "documentation/docs";
static SYNTAX_NAME: &str = "syntax";
static RELEASE_NAME: &str = "release";

pub fn get_root_dir() -> io::Result<PathBuf> {
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

pub fn get_workspace_dir() -> io::Result<PathBuf> {
    let mut bin = get_root_dir()?;
    bin.push(WORKSPACE_NAME);
    if !bin.exists() {
        fs::create_dir(&bin)?;
    }
    Ok(bin)
}

pub fn get_workspace_manifest() -> io::Result<PathBuf> {
    Ok(get_root_dir()?.join(WORKSPACE_FILE))
}

pub fn get_env_file() -> io::Result<PathBuf> {
    let mut env = get_root_dir()?;
    env.push(ENV);
    Ok(env)
}

pub fn get_bin_dir() -> io::Result<PathBuf> {
    let mut bin = get_root_dir()?;
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
    Ok(get_root_dir()?.join(RUNTIME_NAME))
}

pub fn get_cache_dir() -> io::Result<PathBuf> {
    let cache = get_root_dir()?.join(CACHE_NAME);
    if !cache.exists() {
        fs::create_dir(&cache)?;
    }
    Ok(cache)
}

pub fn get_cache_src_dir() -> io::Result<PathBuf> {
    let src = get_cache_dir()?.join(SRC_NAME);
    if !src.exists() {
        fs::create_dir(&src)?;
    }
    Ok(src)
}

pub fn get_registry_dir() -> io::Result<PathBuf> {
    Ok(get_runtime_dir()?.join(REGISTRY_NAME))
}

pub fn get_docs_dir() -> io::Result<PathBuf> {
    Ok(get_runtime_dir()?.join(DOCUMENTATION_NAME))
}

pub fn get_syntax_dir() -> io::Result<PathBuf> {
    Ok(get_runtime_dir()?.join(SYNTAX_NAME))
}

pub fn get_blueprint_dir() -> io::Result<PathBuf> {
    Ok(get_runtime_dir()?.join(BLUEPRINT_NAME))
}

pub fn get_release_dir() -> io::Result<PathBuf> {
    let mut buf = get_root_dir()?;
    buf.push(RELEASE_NAME);
    Ok(buf)
}

pub fn get_release_bin_dir() -> io::Result<PathBuf> {
    let mut buf = get_release_dir()?;
    buf.push(BIN);
    Ok(buf)
}

pub use home;
