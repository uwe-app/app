use std::path::PathBuf;

use log::{error, info};

use site;

use crate::Result;

#[derive(Debug)]
pub struct AddOptions {
    pub name: Option<String>,
    pub project: PathBuf,
}

#[derive(Debug)]
pub struct RemoveOptions {
    pub name: String,
}

pub fn add(options: AddOptions) -> Result<()> {
    let name = site::add(options.project, options.name)?;
    info!("Added {}", &name);
    Ok(())
}

pub fn remove(options: RemoveOptions) -> Result<()> {
    site::remove(&options.name)?;
    info!("Removed {}", &options.name);
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
                error!("{} -> {} [invalid]", name, status.entry.project.display());
            }
        }
    }
    Ok(())
}
