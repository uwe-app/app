use std::fs;
use std::path::PathBuf;

use home;
use crate::Error;

static ROOT_DIR: &str = ".hypertext";
static REPO: &str = "https://github.com/hypertext-live/blueprint";
static BLUEPRINT: &str = "blueprint";

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

pub fn get_blueprint_url() -> String {
    REPO.to_string()
}

pub fn get_blueprint_dir() -> Result<PathBuf, Error> {
    let mut buf = get_root_dir()?;
    buf.push(BLUEPRINT);
    Ok(buf)
}


