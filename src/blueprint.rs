use std::path::Path;
use std::path::PathBuf;
//use std::time::SystemTime;

use git2::{Repository, ErrorClass, ErrorCode};
use log::info;

use crate::Error;
use crate::{cache, git};

// TODO: support --offline to skip attempting to update
// TODO: support blueprint fetch config: always | never

static REPO: &str = "https://github.com/hypertext-live/blueprint";
static BLUEPRINT: &str = "blueprint";
static ORIGIN: &str = "origin";

pub fn get_repo_url() -> String {
    REPO.to_string()
}

pub fn get_repo_dir() -> Result<PathBuf, Error> {
    let mut buf = cache::get_root_dir()?;
    buf.push(BLUEPRINT);
    Ok(buf)
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

pub fn clone_or_fetch() -> Result<(), Error> {
    let dir = get_repo_dir()?;
    let (repo, cloned) = git::open_or_clone(REPO, &dir)?;
    if !cloned {
        //fetch(&repo, &base)?;
        // FIXME: merge from origin/master
        //
        git::pull::pull(&dir, None, None)?;

        fetch_submodules(&repo, &dir)?
    }
    Ok(())
}
