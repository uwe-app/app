use std::io::stderr;
use std::path::Path;

use git2::{build::RepoBuilder, FetchOptions, Repository};

use pbr::ProgressBar;

use crate::{callbacks, Error, Result};

pub(crate) fn clone<S: AsRef<str>, P: AsRef<Path>>(
    src: S,
    target: P,
) -> Result<Repository> {
    let mut callbacks = callbacks::ssh_agent();

    let mut pb = ProgressBar::on(stderr(), 0);
    pb.show_speed = false;
    callbacks.transfer_progress(move |stats| {
        if stats.received_objects() == stats.total_objects() {
            pb.message(" Resolve deltas ");
            pb.total = stats.total_deltas() as u64;
            pb.set(stats.indexed_deltas() as u64);
        } else if stats.total_objects() > 0 {
            pb.message(" Fetch ");
            pb.total = stats.total_objects() as u64;
            pb.set(stats.received_objects() as u64);
        }
        true
    });

    //progress::add_progress_callbacks(&mut callbacks);

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fo);

    let result = builder.clone(src.as_ref(), target.as_ref());
    result.map_err(Error::from)
}

//pub fn clone_recurse<P: AsRef<Path>>(from: &str, dir: P) -> Result<Repository> {
//let repo = match Repository::clone_recurse(from, dir) {
//Ok(repo) => repo,
//Err(e) => return Err(Error::from(e)),
//};
//Ok(repo)
//}

/*
pub fn open_or_clone<P: AsRef<Path>>(
    from: &str,
    to: P,
    recursive: bool,
) -> Result<(Repository, bool)> {
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
*/
