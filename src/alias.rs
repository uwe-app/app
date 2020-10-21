use crate::Result;
use log::{error, info};
use std::path::PathBuf;

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
        info!("No sites yet");
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
