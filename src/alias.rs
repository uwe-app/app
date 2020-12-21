use std::path::PathBuf;

use log::{error, info};

use crate::{opts::Alias, Result};

pub fn add(project: PathBuf, name: Option<String>) -> Result<()> {
    let name = site::add(project, name)?;
    info!("Added {}", &name);
    Ok(())
}

pub fn remove(name: String) -> Result<()> {
    site::remove(&name)?;
    info!("Removed {}", &name);
    Ok(())
}

pub fn list() -> Result<()> {
    let sites = site::list()?;
    if sites.is_empty() {
        info!("No site aliases yet");
    } else {
        for (name, status) in sites {
            if status.ok {
                info!("{} -> {}", name, status.entry.project.display());
            } else {
                error!(
                    "{} -> {} [invalid]",
                    name,
                    status.entry.project.display()
                );
            }
        }
    }
    Ok(())
}

pub async fn run(cmd: Alias) -> Result<()> {
    match cmd {
        Alias::Add { name, project } => {
            add(project, name)?;
        }
        Alias::Remove { name } => {
            remove(name)?;
        }
        Alias::List { .. } => {
            list()?;
        }
    }
    Ok(())
}
