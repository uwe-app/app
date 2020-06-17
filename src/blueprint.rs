use std::fs;
use std::path::Path;
use std::path::PathBuf;
//use std::time::SystemTime;

use git2::Repository;
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

fn get_repo_dir() -> Result<PathBuf, Error> {
    let mut buf = get_root_dir()?;
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

fn fetch_submodules<P: AsRef<Path>>(repo: Repository, base: P) -> Result<(), Error> {
    let modules = repo.submodules()?;
    for sub in modules {
        let mut tmp = base.as_ref().to_path_buf();
        tmp.push(sub.path());
        let repo = match Repository::open(tmp) {
            Ok(repo) => repo,
            Err(e) => return Err(Error::from(e)),
        };

        info!("fetch {} in {}", sub.path().display(), base.as_ref().display());
        repo.find_remote("origin")?.fetch(&["master"], None, None)?;
    }
    Ok(())
}

pub fn list_submodules(repo: Repository) -> Result<(), Error> {
    //let repo = open_repo(get_repo_dir()?)?;
    let modules = repo.submodules()?;
    for sub in &modules {
        info!("{}", sub.path().display());
    }
    Ok(())
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
        fetch_submodules(repo, base)?
    }

    //let mut buf = get_root_dir()?;
    //buf.push(BLUEPRINT);
    //if !buf.exists() {
        ////let now = SystemTime::now();
        //info!("clone {} -> {}", REPO, buf.display());
        //let _ = match Repository::clone_recurse(REPO, buf) {
            //Ok(repo) => repo,
            //Err(e) => return Err(Error::from(e)),
        //};
        ////if let Ok(t) = now.elapsed() {
            ////info!("done {:?}", t);
        ////}

    //} else {
        //if buf.is_dir() {
            //// TODO: support --offline to skip attempting to update
            //// TODO: support blueprint fetch config: always | never
            //fetch_submodules(&buf)?;
        //} else {
            //return Err(Error::new(format!("Not a directory {}", buf.display())));
        //}
    //}

    Ok(())
}
