use std::fs;
use std::path::{Path, PathBuf};

use git2::{
    Commit, IndexAddOption, Oid, PushOptions, RemoteCallbacks, Repository,
    RepositoryInitOptions, RepositoryState, StatusOptions,
};

use log::{info, warn};
use thiserror::Error;

pub static HEAD: &str = "HEAD";
pub static ORIGIN: &str = "ORIGIN";
pub static MAIN: &str = "MAIN";
pub static REFSPEC: &str = "refs/heads/main:refs/head/main";

#[derive(Error, Debug)]
pub enum Error {
    #[error("No commit available")]
    NoCommit,

    #[error("Conflict detected in {0}, please resolve manually")]
    Conflict(PathBuf),

    #[error("Remote {0} does not exist in the repository {1}")]
    NoRemote(String, PathBuf),

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

mod callbacks;
mod clone;
//mod progress;
mod pull;

pub fn pull<P: AsRef<Path>>(
    path: P,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<()> {
    let remote_name = remote.as_ref().map(|s| &s[..]).unwrap_or(ORIGIN);
    let branch_name = branch.as_ref().map(|s| &s[..]).unwrap_or(MAIN);

    info!(
        "Pull {}/{} in {}",
        remote_name,
        branch_name,
        path.as_ref().display()
    );

    pull::pull(path, remote_name, branch_name).map_err(Error::from)
}

pub fn clone<S: AsRef<str>, P: AsRef<Path>>(
    src: S,
    target: P,
) -> Result<Repository> {
    Ok(clone::clone(src, target).map_err(Error::from)?)
}

pub fn copy<S: AsRef<str>, P: AsRef<Path>>(
    src: S,
    target: P,
    message: &str,
) -> Result<Repository> {
    let target = target.as_ref();
    let repo = clone(src, target).map_err(Error::from)?;
    pristine(target, &repo, message)?;
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
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head(MAIN);

    // Create fresh repository
    let new_repo = Repository::init_opts(target.as_ref(), &opts)?;

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
    Ok(new_repo.commit(Some(HEAD), &sig, &sig, message, &tree, parents)?)
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
    let oid = add_files(repo, &[path])?;
    Ok(commit(repo, Some(HEAD), oid, message)?)
}

/// Make a commit using `oid` as the current tip.
pub fn commit(
    repo: &Repository,
    update_ref: Option<&str>,
    oid: Oid,
    message: &str,
    ) -> Result<Oid> {

    let sig = repo.signature()?;
    let tree = repo.find_tree(oid)?;
    let tip = find_last_commit(repo)?;
    let commit = tip.ok_or_else(|| Error::NoCommit)?;

    let parents: [&Commit; 1] = [&commit];
    Ok(repo.commit(update_ref, &sig, &sig, message, &tree, &parents)?)
}

/// Add files to the index and write the tree.
pub fn add_files(
    repo: &Repository,
    paths: &[&Path],
    ) -> Result<Oid> {

    let mut index = repo.index()?;
    for p in paths {
        index.add_path(p)?;
    }
    // TODO: check for conflicts index.has_conflicts()?;
    index.write()?;
    Ok(index.write_tree()?)
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

/*
fn fetch<P: AsRef<Path>>(repo: &Repository, base: P) -> Result<()> {
    info!("Fetch {}", base.as_ref().display());
    repo.find_remote(ORIGIN)?
        .fetch(&[MAIN], None, None)
        .map_err(Error::from)
}
*/

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

pub fn discover<P: AsRef<Path>>(dir: P) -> Result<Repository> {
    Ok(Repository::discover(dir).map_err(Error::from)?)
}

pub fn is_clean(repo: &Repository) -> bool {
    repo.state() == RepositoryState::Clean
}

pub fn clone_or_fetch<P: AsRef<Path>>(from: &str, to: P) -> Result<Repository> {
    let to = to.as_ref();
    if !to.exists() {
        print_clone(from, to);
        Ok(clone(from, to)?)
    } else {
        let repo = open(to)?;
        pull(to, None, None)?;
        Ok(repo)
    }
}

pub fn last_commit(
    repo: &Repository,
    spec: &str,
    ) -> Option<Oid> {
    if let Some(rev) = repo.revparse(spec).ok() {
        if let Some(obj) = rev.from() {
            if let Some(commit) = obj.as_commit() {
                return Some(commit.id());
            }
        }
    }
    None
}

/// Sync a project with a remote repository.
pub fn sync<P: AsRef<Path>>(
    dir: P,
    remote: String,
    branch: String,
    add_untracked: bool,
    message: Option<String>,
    ) -> Result<()> {

    let repo = open(dir.as_ref())?;

    let _ = repo.find_remote(&remote)
        .map_err(|_| Error::NoRemote(remote.to_string(), dir.as_ref().to_path_buf()))?;

    // Make sure the repository has a commit
    let last_commit: Oid = last_commit(&repo, HEAD)
        .ok_or(Error::NoCommit)?;

    let tip = repo.find_commit(last_commit)?;
    let mut tree_id = tip.tree_id();
    let mut commit_required = false;
    let mut changed_files: Vec<String> = Vec::new();

    // 1) Check status to add untracked files and
    //    determine if a commit is needed.
    let mut status_options = StatusOptions::new();
    status_options.include_untracked(true);
    let statuses = repo.statuses(Some(&mut status_options))?;
    if !statuses.is_empty() {
        for entry in statuses.iter() {
            let status = entry.status();

            if status.is_conflicted() {
                return Err(Error::Conflict(dir.as_ref().to_path_buf()))
            }

            if status.is_wt_new() {
                if let Some(path) = entry.path() {
                    if add_untracked {
                        info!("Add file {}", path);
                        changed_files.push(path.to_string());
                        commit_required = true;
                    } else {
                        warn!("Skip file {}", path);
                    }
                }
            } else if status.is_wt_modified()
                || status.is_wt_deleted()
                || status.is_wt_typechange()
                || status.is_wt_renamed() {
                if let Some(path) = entry.path() {
                    changed_files.push(path.to_string());
                    commit_required = true;
                }
            } else if status.is_index_new()
                || status.is_index_modified()
                || status.is_index_deleted()
                || status.is_index_typechange()
                || status.is_index_renamed() {
                commit_required = true;
            }
        }
    }

    if !changed_files.is_empty() {
        let files: Vec<&Path> = changed_files
            .iter()
            .map(|p| Path::new(p))
            .collect();
        tree_id = add_files(&repo, files.as_slice())?;
    }

    // 2) Perform the commit if we have a commit required
    //    and a commit message is available.
    if commit_required {
        if let Some(ref message) = message {
            info!("Commit {:?}", message);
            commit(&repo, Some(HEAD), tree_id, message)?;
        } else {
            if !changed_files.is_empty() {
                warn!("Changed files detected but no commit performed ");
                warn!("because a commit message is not available.");
            }
        }
    }

    // 3) Pull the remote repository
    // TODO: Handle merge conflicts on the pull???
    pull(dir.as_ref(), Some(&remote), Some(&branch))?;

    // 4) Push to the remote repository
    push(&repo, &remote, None, None)?;

    Ok(())
}
