use std::path::Path;
use std::path::PathBuf;
//use std::time::SystemTime;

use git2::{Repository, ErrorClass, ErrorCode};
use log::info;

use crate::Error;
use crate::preference;
use crate::utils;

// TODO: support --offline to skip attempting to update
// TODO: support blueprint fetch config: always | never

static REPO: &str = "https://github.com/hypertext-live/blueprint";
static BLUEPRINT: &str = "blueprint";
static ORIGIN: &str = "origin";

pub fn get_repo_dir() -> Result<PathBuf, Error> {
    let mut buf = preference::get_root_dir()?;
    buf.push(BLUEPRINT);
    Ok(buf)
}

fn open_repo<P: AsRef<Path>>(dir: P) -> Result<Repository, Error> {
    let repo = match Repository::open(dir) {
        Ok(repo) => repo,
        Err(e) => return Err(Error::from(e)),
    };
    Ok(repo)
}

fn clone_repo<P: AsRef<Path>>(dir: P) -> Result<Repository, Error> {
    let repo = match Repository::clone_recurse(REPO, dir) {
        Ok(repo) => repo,
        Err(e) => return Err(Error::from(e)),
    };
    Ok(repo)
}

fn fetch_submodules<P: AsRef<Path>>(repo: &Repository, base: P) -> Result<(), Error> {
    let modules = repo.submodules()?;
    for mut sub in modules {

        sub.sync()?;

        let mut tmp = base.as_ref().to_path_buf();
        tmp.push(sub.path());

        //println!("Trying to fetch with {:?}", sub.url());
        //println!("Trying to fetch with {:?}", tmp.display());
        //println!("Trying to fetch with {:?}", tmp.exists());

        match sub.open() {
            Ok(repo) => {
                fetch(&repo, sub.path())?;
            },
            Err(e) => {
                if let ErrorClass::Os = e.class() {
                    if let ErrorCode::NotFound = e.code() {
                        if let Some(ref url) = sub.url() {
                            info!("Clone {}", url);
                            info!("   -> {}", tmp.display());
                            Repository::clone(url, tmp)?;
                        }

                    }
                }
                return Err(Error::from(e));
            },
        };
    }
    Ok(())
}

fn fetch<P: AsRef<Path>>(repo: &Repository, base: P) -> Result<(), Error> {
    info!("fetch {}", base.as_ref().display());
    repo.find_remote(ORIGIN)?.fetch(&["master"], None, None).map_err(Error::from)
}

pub fn list_submodules(repo: Repository) -> Result<(), Error> {
    //let repo = open_repo(get_repo_dir()?)?;
    let modules = repo.submodules()?;
    for sub in &modules {
        info!("{}", sub.path().display());
    }
    Ok(())
}

pub fn will_clone() -> Result<(bool, PathBuf, String), Error> {
    let buf = get_repo_dir()?;
    Ok((!buf.exists(), buf, REPO.to_string()))
}

pub fn open_or_clone() -> Result<(Repository, PathBuf, bool), Error> {
    let buf = get_repo_dir()?;
    if !buf.exists() {
        let repo = clone_repo(&buf)?;
        return Ok((repo, buf, true))
    } else {
        let repo = open_repo(&buf)?;
        return Ok((repo, buf, false))
    }
}

pub fn clone_or_fetch() -> Result<(), Error> {
    let (repo, base, cloned) = open_or_clone()?;
    if !cloned {
        //fetch(&repo, &base)?;
        // FIXME: merge from origin/master
        //
        utils::git_pull::pull(&base, None, None)?;

        fetch_submodules(&repo, &base)?
    }
    Ok(())
}
