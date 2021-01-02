use std::path::PathBuf;

use log::info;

use crate::{
    opts::{self, Task},
    Error, Result,
};
use config::plugin::dependency::DependencyTarget;

use super::alias;

pub async fn run(cmd: Task) -> Result<()> {
    match cmd {
        Task::ListBlueprints {} => {
            list_blueprints().await?;
        }
        Task::CheckDeps { project } => {
            let project = opts::project_path(&project)?;
            check_deps(project).await?;
        }
        Task::Alias { cmd } => {
            alias::run(cmd).await?;
        }
        Task::UpdateRuntime {} => {
            update_runtime().await?;
        } /*
          Task::Pull {
              project,
              remote,
              branch,
          } => {
              pull(project, remote, branch).await?;
          }
          */
    }
    Ok(())
}

/// List standard blueprints.
async fn list_blueprints() -> Result<()> {
    let blueprints = dirs::blueprint_dir()?;
    for entry in std::fs::read_dir(blueprints)? {
        let path = entry?.path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy();
            info!("{} ({})", name, path.display());
        }
    }
    Ok(())
}

/// Check plugin dependencies do not use `path` or `archive`
/// local references.
async fn check_deps(project: PathBuf) -> Result<()> {
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    let workspace = workspace::open(&project, true, &vec![])?;
    for config in workspace.into_iter() {
        if let Some(deps) = config.dependencies {
            for (name, dep) in deps.iter() {
                if let Some(ref target) = dep.target {
                    match target {
                        DependencyTarget::File { path } => {
                            return Err(Error::LocalDependencyNotAllowed(
                                path.to_path_buf(),
                            ))
                        }
                        DependencyTarget::Archive { archive } => {
                            return Err(Error::LocalDependencyNotAllowed(
                                archive.to_path_buf(),
                            ))
                        }
                        _ => {}
                    }
                }
                info!("Dependency {} is ok ✓", name)
            }
        }
    }

    Ok(())
}

/// Update the runtime assets.
pub async fn update_runtime() -> Result<()> {
    let url = dirs::runtime_url();
    let dir = dirs::runtime_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}

/*
/// Pull from a remote repository.
async fn pull(
    target: PathBuf,
    remote: String,
    branch: String,
) -> Result<()> {
    let target = opts::project_path(&target)?;
    if !target.exists() || !target.is_dir() {
        return Err(Error::NotDirectory(target));
    }

    scm::open(&target)
        .map_err(|_| Error::NotRepository(target.to_path_buf()))?;

    scm::pull(&target, Some(remote), Some(branch))
        .map(|_| ())
        .map_err(Error::from)
}
*/

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
