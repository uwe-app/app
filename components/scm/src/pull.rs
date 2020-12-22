/*
 * libgit2 "pull" example - shows how to pull remote data into a local branch.
 *
 * Written by the libgit2 contributors
 *
 * To the extent possible under law, the author(s) have dedicated all copyright
 * and related and neighboring rights to this software to the public domain
 * worldwide. This software is distributed without any warranty.
 *
 * You should have received a copy of the CC0 Public Domain Dedication along
 * with this software. If not, see
 * <http://creativecommons.org/publicdomain/zero/1.0/>.
 */

use std::io::stderr;
use std::path::Path;

use git2::Repository;
use log::{debug, info};

use pbr::ProgressBar;

use crate::callbacks;

fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
    remote_name: &'a str,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = callbacks::ssh_agent();
    let mut pb = ProgressBar::on(stderr(), 0);
    pb.show_speed = false;

    cb.transfer_progress(move |stats| {
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

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb);

    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);

    debug!("Fetching {}", remote.name().unwrap());
    remote.fetch(refs, Some(&mut fo), None)?;

    //let stats = remote.stats();
    //if stats.received_bytes() > 0 {
    //let _ = clear_progress_bar();
    //}

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    //let stats = remote.stats();

    let fetch_ref = format!("refs/remotes/{}/{}", remote_name, refs[0]);
    let fetch_head = repo.find_reference(&fetch_ref)?;
    Ok(repo.reference_to_annotated_commit(&fetch_head)?)
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };

    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    debug!("{}", msg);

    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))?;
    Ok(())
}

fn normal_merge(
    repo: &Repository,
    local: &git2::AnnotatedCommit,
    remote: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx =
        repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        info!("Merge conficts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appopriate merge
    if analysis.0.is_fast_forward() {
        debug!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!(
                        "Setting {} to {}",
                        remote_branch,
                        fetch_commit.id()
                    ),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(&repo, &head_commit, &fetch_commit)?;
    } else {
        debug!("No merge needed");
    }
    Ok(())
}

pub(crate) fn pull<P: AsRef<Path>>(
    path: P,
    remote_name: &str,
    branch_name: &str,
) -> Result<(), git2::Error> {
    /*
    let remote_name = remote.as_ref().map(|s| &s[..]).unwrap_or("origin");
    let remote_branch = branch.as_ref().map(|s| &s[..]).unwrap_or("main");

    info!(
        "Pull {}/{} in {}",
        remote_name,
        remote_branch,
        path.as_ref().display()
    );
    */

    let repo = Repository::open(path)?;
    let mut remote = repo.find_remote(remote_name)?;
    let fetch_commit =
        do_fetch(&repo, &[branch_name], &mut remote, &remote_name)?;
    do_merge(&repo, &branch_name, fetch_commit)
}
