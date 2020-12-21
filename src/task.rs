use std::path::PathBuf;

use log::info;

use crate::{Error, Result};
use config::plugin::dependency::DependencyTarget;

/// List standard blueprints.
pub async fn list_blueprints() -> Result<()> {
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
pub async fn check_deps(project: PathBuf) -> Result<()> {
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    let workspace = workspace::open(&project, true)?;
    for entry in workspace.into_iter() {
        if let Some(deps) = entry.config.dependencies {
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
