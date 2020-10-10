use std::fs;
use std::path::Path;

use git2::{
    Commit,
    IndexAddOption, Oid, PushOptions, RemoteCallbacks, Repository,
    RepositoryState,
};

use log::info;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No commit available")]
    NoCommit,

    //#[error("Unable to handle source {0}")]
    //BadSource(String),

    //#[error("To use SSH specify the --private-key option")]
    //PrivateKeyRequired,

    #[error(transparent)]
    Git(#[from] git2::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;

//pub mod bridge;
mod callbacks;
mod clone;
mod progress;
mod pull;

pub static ORIGIN: &str = "origin";
pub static MASTER: &str = "master";
pub static REFSPEC: &str = "refs/heads/master:refs/head/master";

pub fn pull<P: AsRef<Path>>(
    path: P,
    remote: Option<String>,
    branch: Option<String>,
) -> Result<()> {
    pull::pull(path, remote, branch)
        .map_err(Error::from)
}

pub fn clone<S: AsRef<str>, P: AsRef<Path>>(
    src: S,
    target: P,
    message: Option<&str>,
) -> Result<Repository> {
    let target = target.as_ref();

    let repo = clone::clone(src, target)
        .map_err(Error::from)?;

    if let Some(message) = message {
        pristine(target, &repo, message)? 
    }

    Ok(repo)
}

/// Detach a repository from upstream by removing the entire commit
/// history and creating a fresh repository.
pub fn pristine<P: AsRef<Path>>(
    target: P,
    repo: &Repository,
    message: &str,
) -> Result<()> {
    let git_dir = repo.path();

    // Remove the git directory is the easiest
    // way to purge the history
    fs::remove_dir_all(git_dir)?;

    init(target, message)?;
    Ok(())
}

/// Initialize a repository and perform an initial commit.
pub fn init<P: AsRef<Path>>(target: P, message: &str) -> Result<Oid> {
    // Create fresh repository
    let new_repo = Repository::init(target.as_ref())?;

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

    let sig = new_repo.signature()?;
    let tree = new_repo.find_tree(oid)?;
    let parents: &[&Commit] = &[];
    Ok(new_repo.commit(Some("HEAD"), &sig, &sig, message, &tree, parents)?)
}

pub fn find_last_commit<'a>(
    repo: &'a Repository,
) -> Result<Option<Commit<'a>>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    for rev in revwalk {
        let commit = repo.find_commit(rev?)?;
        return Ok(Some(commit));
    }
    Ok(None)
}

pub fn push(
    repo: &Repository,
    remote: &str,
    cbs: Option<RemoteCallbacks<'_>>,
    refspecs: Option<Vec<String>>,
) -> Result<()> {
    let mut cbs = cbs.unwrap_or(callbacks::ssh_agent());
    let mut remote = repo.find_remote(remote)?;

    let refspecs = refspecs.unwrap_or({
        remote
            .push_refspecs()?
            .iter()
            .collect::<Vec<_>>()
            .iter()
            .map(|s| s.unwrap().to_string())
            .collect::<Vec<_>>()
    });

    let refspecs = if !refspecs.is_empty() {
        refspecs
    } else {
        vec![REFSPEC.to_string()]
    };

    //cbs.push_transfer_progress(|obj_sent, obj_total, bytes| {});

    cbs.push_update_reference(|name, status| {
        if status.is_none() {
            info!("Pushed {}", name);
        }
        Ok(())
    });

    //println!("Remote {:?}", remote.pushurl());
    //println!("Remote {:?}", remote.name());
    //println!("Refspecs {:#?}", refspecs);

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(cbs);
    let opts = Some(&mut push_options);
    remote.push(&refspecs.as_slice(), opts)?;

    Ok(())
}

/// Add and commit a file; the path must be relative to the repository.
pub fn commit_file(
    repo: &Repository,
    path: &Path,
    message: &str,
) -> Result<Oid> {
    let sig = repo.signature()?;
    let mut index = repo.index()?;
    index.add_path(path)?;

    // TODO: check for conflicts index.has_conflicts()?;

    index.write()?;
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    let tip = find_last_commit(repo)?;
    let commit = tip.ok_or_else(|| Error::NoCommit)?;

    let parents: [&Commit; 1] = [&commit];
    Ok(repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?)
}

//pub fn clone<S: AsRef<str>, P: AsRef<Path>>(
    //src: S,
    //target: P,
//) -> Result<Repository> {
    //let mut callbacks = callbacks_ssh_agent();
    //progress::add_progress_callbacks(&mut callbacks);

    //let mut fo = FetchOptions::new();
    //fo.remote_callbacks(callbacks);

    //let mut builder = RepoBuilder::new();
    //builder.fetch_options(fo);

    //let result = builder.clone(src.as_ref(), target.as_ref());
    //result.map_err(Error::from)
//}

/*
pub fn clone_standard<P: AsRef<Path>>(
    src: &str,
    target: P,
) -> Result<Repository> {
    let mut callbacks = RemoteCallbacks::new();
    progress::add_progress_callbacks(&mut callbacks);

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fo);

    builder.clone(src, target.as_ref()).map_err(Error::from)
}
*/

/*
fn fetch_submodules<P: AsRef<Path>>(repo: &Repository, base: P) -> Result<()> {
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
*/

fn fetch<P: AsRef<Path>>(repo: &Repository, base: P) -> Result<()> {
    info!("Fetch {}", base.as_ref().display());
    repo.find_remote(ORIGIN)?
        .fetch(&["master"], None, None)
        .map_err(Error::from)
}

pub fn print_clone<P: AsRef<Path>>(from: &str, to: P) {
    info!("Clone {}", from);
    info!("   -> {}", to.as_ref().display());
}

/*
pub fn list_submodules(repo: Repository) -> Result<()> {
    let modules = repo.submodules()?;
    for sub in &modules {
        info!("{}", sub.path().display());
    }
    Ok(())
}
*/

pub fn open<P: AsRef<Path>>(dir: P) -> Result<Repository> {
    Ok(Repository::open(dir).map_err(Error::from)?)
}

pub fn is_clean(repo: &Repository) -> bool {
    repo.state() == RepositoryState::Clean
}

pub fn clone_or_fetch<P: AsRef<Path>>(
    from: &str,
    to: P,
) -> Result<Repository> {
    let to = to.as_ref();
    if !to.exists() {
        print_clone(from, to);
        Ok(clone(from, to, None)?)
    } else {
        let repo = open(to)?;
        pull(to, None, None)?;
        Ok(repo)
    }

    //let repo = open_repo(to)?;

    //let (repo, cloned) = open_or_clone(from, to.as_ref(), submodules)?;
    //if !cloned {
        //fetch(&repo, &base)?;
        // FIXME: merge from origin/master

    //pull(to, None, None)?;
        //if submodules {
            //fetch_submodules(&repo, to.as_ref())?
        //}
    //}

    //Ok(repo)
}
