use std::fs;
use std::path::Path;
use std::path::PathBuf;

use git2::build::RepoBuilder;
use git2::{
    Commit, Cred, ErrorClass, ErrorCode, FetchOptions, IndexAddOption, RemoteCallbacks, Repository,
};

use log::info;

//use crate::Error;

pub mod progress;
pub mod pull;
pub mod error;

use error::{GitError as Error};

static ORIGIN: &str = "origin";

pub fn detached<P: AsRef<Path>>(target: P, repo: Repository) -> Result<(), Error> {
    let git_dir = repo.path();

    // Remove the git directory is the easiest
    // way to purge the history
    fs::remove_dir_all(git_dir)?;

    // Create fresh repository
    let new_repo = Repository::init(target)?;

    // Add all the files
    let mut index = new_repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    // NOTE: must call `write` and `write_tree`
    index.write()?;
    let oid = index.write_tree()?;

    // TODO: get these from preferences when not setand use defaults otherwise
    //let conf = Config::open_default()?;
    //let name = conf.get_string("user.name")?;
    //let email = conf.get_string("user.email")?;

    let sig = repo.signature()?;

    // TODO: allow prefernce for this
    let message = "Initial files.";

    let tree = new_repo.find_tree(oid)?;
    let parents: &[&Commit] = &[];

    let _commit_id = new_repo.commit(Some("HEAD"), &sig, &sig, message, &tree, parents)?;

    Ok(())
}

pub fn clone_ssh<P: AsRef<Path>>(
    src: String,
    target: P,
    key_file: PathBuf,
    password: Option<String>,
) -> Result<Repository, Error> {
    let passphrase = if let Some(ref phrase) = password {
        Some(phrase.as_str())
    } else {
        None
    };

    let private_key = key_file.as_path();

    let mut callbacks = RemoteCallbacks::new();
    progress::add_progress_callbacks(&mut callbacks);
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(username_from_url.unwrap(), None, private_key, passphrase)
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fo);

    let result = builder.clone(&src, target.as_ref());

    if let Err(ref e) = result {
        if let ErrorClass::Ssh = e.class() {
            // Sadly cannot find a better way to detect this particular error
            if e.message().contains("Wrong passphrase") {
                let pass = rpassword::read_password_from_tty(Some("Passphrase: "))?;
                return clone_ssh(src, target, key_file.clone(), Some(pass));
            }
        }
    }

    result.map_err(Error::from)
}

pub fn clone_standard<P: AsRef<Path>>(src: &str, target: P) -> Result<Repository, Error> {
    let mut callbacks = RemoteCallbacks::new();
    progress::add_progress_callbacks(&mut callbacks);

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fo);

    builder.clone(src, target.as_ref()).map_err(Error::from)
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
            }
            Err(e) => {
                if let ErrorClass::Os = e.class() {
                    if let ErrorCode::NotFound = e.code() {
                        if let Some(ref url) = sub.url() {
                            print_clone(&url, &tmp);
                            Repository::clone(url, tmp)?;
                        }
                    }
                }
                return Err(Error::from(e));
            }
        };
    }
    Ok(())
}

fn fetch<P: AsRef<Path>>(repo: &Repository, base: P) -> Result<(), Error> {
    info!("Fetch {}", base.as_ref().display());
    repo.find_remote(ORIGIN)?
        .fetch(&["master"], None, None)
        .map_err(Error::from)
}

pub fn print_clone<P: AsRef<Path>>(from: &str, to: P) {
    info!("Clone {}", from);
    info!("   -> {}", to.as_ref().display());
}

pub fn list_submodules(repo: Repository) -> Result<(), Error> {
    let modules = repo.submodules()?;
    for sub in &modules {
        info!("{}", sub.path().display());
    }
    Ok(())
}

pub fn open_repo<P: AsRef<Path>>(dir: P) -> Result<Repository, Error> {
    let repo = match Repository::open(dir) {
        Ok(repo) => repo,
        Err(e) => return Err(Error::from(e)),
    };
    Ok(repo)
}

//pub fn clone_repo<P: AsRef<Path>>(from: &str, dir: P) -> Result<Repository, Error> {
//let repo = match Repository::clone(from, dir) {
//Ok(repo) => repo,
//Err(e) => return Err(Error::from(e)),
//};
//Ok(repo)
//}

pub fn clone_recurse<P: AsRef<Path>>(from: &str, dir: P) -> Result<Repository, Error> {
    let repo = match Repository::clone_recurse(from, dir) {
        Ok(repo) => repo,
        Err(e) => return Err(Error::from(e)),
    };
    Ok(repo)
}

pub fn open_or_clone<P: AsRef<Path>>(
    from: &str,
    to: P,
    submodules: bool,
) -> Result<(Repository, bool), Error> {
    if !to.as_ref().exists() {
        let repo = if submodules {
            clone_recurse(from, to)?
        } else {
            clone_standard(from, to)?
        };
        return Ok((repo, true));
    } else {
        let repo = open_repo(to)?;
        return Ok((repo, false));
    }
}

pub fn clone_or_fetch<P: AsRef<Path>>(from: &str, to: P, submodules: bool) -> Result<(), Error> {
    if !to.as_ref().exists() {
        print_clone(from, to.as_ref().clone());
    }

    let (repo, cloned) = open_or_clone(from, to.as_ref(), submodules)?;
    if !cloned {
        //fetch(&repo, &base)?;
        // FIXME: merge from origin/master

        pull::pull(to.as_ref(), None, None)?;
        if submodules {
            fetch_submodules(&repo, to.as_ref())?
        }
    }
    Ok(())
}

//#[cfg(test)]
//mod tests {
    //#[test]
    //fn it_works() {
        //assert_eq!(2 + 2, 4);
    //}
//}
