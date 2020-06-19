use std::fs;
use std::path::PathBuf;

use home;
use crate::Error;

static ROOT_DIR: &str = ".hypertext";

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

