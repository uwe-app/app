//use std::path::PathBuf;
//use url::Url;
use crate::{opts::{self, Sync}, Error, Result};

/*
fn create(target: PathBuf, message: String) -> Result<()> {
    let target = opts::project_path(&target)?;
    if !target.exists() || !target.is_dir() {
        return Err(Error::NotDirectory(target));
    }

    scm::init(&target, &message)
        .map(|_| ())
        .map_err(Error::from)
}

fn clone_or_copy(
    source: String,
    target: Option<PathBuf>,
    pristine: Option<String>,
) -> Result<()> {

    let target = if let Some(target) = target {
        target.to_path_buf()
    } else {
        let base = std::env::current_dir()?;

        let mut target_parts =
            source.trim_end_matches("/").split("/").collect::<Vec<_>>();

        let target_name =
            target_parts.pop().ok_or_else(|| Error::NoTargetName)?;
        base.join(target_name)
    };

    let _ = source
        .parse::<Url>()
        .map_err(|_| Error::InvalidRepositoryUrl(source.to_string()))?;

    if target.exists() {
        return Err(Error::TargetExists(target.to_path_buf()));
    }

    if let Some(ref message) = pristine {
        scm::copy(&source, &target, message)
            .map(|_| ())
            .map_err(Error::from)
    } else {
        scm::clone(&source, &target)
            .map(|_| ())
            .map_err(Error::from)
    }
}
*/

/*
fn pull(
    target: PathBuf,
    remote: String,
    branch: String,
) -> Result<()> {
    let target = opts::project_path(&target)?;
    if !target.exists() || !target.is_dir() {
        return Err(Error::NotDirectory(target));
    }

    // TODO: open project config
    // TODO: use sync branch from config

    scm::open(&target)
        .map_err(|_| Error::NotRepository(target.to_path_buf()))?;

    scm::pull(&target, Some(remote), Some(branch))
        .map(|_| ())
        .map_err(Error::from)
}
*/

pub async fn run(opts: Sync) -> Result<()> {
    let project = opts::project_path(&opts.project)?;
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    println!("Sync command is running....");

    /*
    if let Some(cmd) = cmd {
        match cmd {

            Sync::Pull {
                project,
                remote,
                branch,
            } => {
                pull(project, remote, branch)
            }
        }
    } else {
        println!("Run the default command...");
        Ok(())
    }
    */

    Ok(())
}
