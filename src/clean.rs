use std::fs;
use std::path::PathBuf;

use log::info;

use crate::{Error, Result};

pub async fn clean(project: PathBuf) -> Result<()> {
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    let workspace = workspace::open(&project, true, &vec![])?;
    for config in workspace.into_iter() {
        let profile = config.build.as_ref().unwrap();
        let target = &profile.target;
        if target.exists() && target.is_dir() {
            info!("Remove {}", target.display());
            fs::remove_dir_all(&target)?;
        }
    }
    Ok(())
}
