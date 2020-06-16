use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use git2::Repository;
use home;
use log::info;

use crate::Error;

static ROOT_DIR: &str = ".hypertext";
static BLUEPRINT: &str = "blueprint";

fn get_root_dir() -> Result<PathBuf, Error> {
    let cache = home::home_dir();
    if let Some(ref cache) = cache {
        let mut buf = cache.clone();
        buf.push(ROOT_DIR);
        if !buf.exists() {
            fs::create_dir_all(&buf)?;
        }
        return Ok(buf);
    }
    Err(
        Error::new(
            format!("Could not determine home directory")))
}

pub fn clone_or_fetch() -> Result<(), Error> {
    let mut buf = get_root_dir()?;
    buf.push(BLUEPRINT);
    if !buf.exists() {
        let url = "https://github.com/hypertext-live/blueprint";
        let now = SystemTime::now();
        info!("clone {} -> {}", url, buf.display());
        let repo = match Repository::clone_recurse(url, buf) {
            Ok(repo) => repo,
            Err(e) => return Err(Error::from(e)),
        };
        if let Ok(t) = now.elapsed() {
            info!("done {:?}", t);
        }
    } else {
        println!("TRY TO PULL LATEST BLUEPRINTS");
    }

    Ok(())
}
