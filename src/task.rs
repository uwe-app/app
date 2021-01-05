use std::path::PathBuf;

use log::info;

use crate::{
    opts::{self, Task},
    Error, Result,
};
use config::plugin::dependency::DependencyTarget;
use plugin::new_registry;

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
        Task::UpdateRegistry {} => {
            update_registry().await?;
        } 
        Task::UpdateBlueprints {} => {
            update_blueprints().await?;
        } 
        Task::UpdateDocs {} => {
            update_docs().await?;
        } 
        Task::UpdateSyntax {} => {
            update_syntax().await?;
        } 
    }
    Ok(())
}

/// Update the plugin registry cache
pub async fn update_registry() -> Result<()> {
    scm::system_repo::fetch_registry().await?;
    info!("Update complete ✓");
    Ok(())
}

/// Update the project blueprints.
pub async fn update_blueprints() -> Result<()> {
    scm::system_repo::fetch_blueprints().await?;
    info!("Update complete ✓");
    Ok(())
}

/// Update the documentation
pub async fn update_docs() -> Result<()> {
    scm::system_repo::fetch_documentation().await?;
    info!("Update complete ✓");
    Ok(())
}

/// Update the syntax highlight language definitions
pub async fn update_syntax() -> Result<()> {
    scm::system_repo::fetch_syntax().await?;
    info!("Update complete ✓");
    Ok(())
}

/// List standard blueprints.
async fn list_blueprints() -> Result<()> {
    let namespace = config::PLUGIN_BLUEPRINT_NAMESPACE;
    let registry = new_registry()?;
    let entries = registry.starts_with(namespace).await?;
    for (name, entry) in entries.iter() {
        let (version, item) = entry.latest().unwrap();
        let short_name = item.short_name().unwrap();
        info!("{} ({}@{})", short_name, name, version.to_string());
    }
    Ok(())
}

/// Check plugin dependencies do not use `path` or `archive`
/// local references.
async fn check_deps(project: PathBuf) -> Result<()> {
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

/*
/// Pull from a remote repository.
async fn pull(
    target: PathBuf,
    remote: String,
    branch: String,
) -> Result<()> {
    let target = opts::project_path(&target)?;

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
