use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use git2::{Repository, Submodule};
use home;
use log::info;

use crate::Error;

static REPO: &str = "https://github.com/hypertext-live/blueprint";
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

pub fn clone_or_fetch<'a>() -> Result<Vec<Box<Submodule<'a>>>, Error> {
    let mut buf = get_root_dir()?;
    buf.push(BLUEPRINT);
    if !buf.exists() {
        let now = SystemTime::now();
        info!("clone {} -> {}", REPO, buf.display());
        let repo = match Repository::clone_recurse(REPO, buf) {
            Ok(repo) => repo,
            Err(e) => return Err(Error::from(e)),
        };
        if let Ok(t) = now.elapsed() {
            info!("done {:?}", t);
        }

        let modules = repo.submodules()?
            .into_iter()
            .map(Box::new)
            .collect::<Vec<_>>();

        return Ok(modules);
    } else {
        if buf.is_dir() {
            // TODO: support --offline to skip attempting to update
            // TODO: support blueprint fetch config: always | never
            
            let repo = match Repository::open(&buf) {
                Ok(repo) => repo,
                Err(e) => return Err(Error::from(e)),
            };

            let modules = repo.submodules()?;
            for sub in &modules {
                let mut tmp = buf.clone();
                tmp.push(sub.path());
                let repo = match Repository::open(tmp) {
                    Ok(repo) => repo,
                    Err(e) => return Err(Error::from(e)),
                };

                info!("fetch {} in {}", sub.path().display(), buf.display());
                repo.find_remote("origin")?.fetch(&["master"], None, None)?;
            }

            let modules = modules
                .into_iter()
                .map(Box::new)
                .collect::<Vec<_>>();

            return Ok(modules)
        } else {
            return Err(Error::new(format!("Not a directory {}", buf.display())));
        }
    }
}
