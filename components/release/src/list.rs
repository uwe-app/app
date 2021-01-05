use log::{info, warn};

use crate::{releases, version, Error, Result};

/// List versions.
pub async fn list() -> Result<()> {
    scm::system_repo::fetch_releases().await?;

    // Load the releases manifest
    let releases = releases::mount()?;

    // Get the current version
    let version_file = version::file()?;
    if !version_file.exists() {
        return Err(Error::NotInstalled);
    }

    let cwd = std::env::current_dir()?;
    let (mut local_version, local_version_file) = version::find_local_version(cwd)?;

    let current = if let Some(version) = local_version.take() {
        version 
    } else { version::default_version()? };

    let total = releases.versions.iter().count();

    info!("-------------------------------");
    info!("| ◯ (installed) | ✓ (current) |");
    info!("-------------------------------");
    info!("");

    for (version, _) in releases.versions.iter().rev() {
        let version_dir = releases::dir(version)?;
        let is_installed = version_dir.exists() && version_dir.is_dir();
        let mark = if is_installed { "◯" } else { "-" };
        if &current == version {
            let message = if let Some(ref file) = local_version_file {
                format!("{} {} ✓ (set by {})", mark, version.to_string(), file.display())
            } else {
                format!("{} {} ✓", mark, version.to_string())
            };

            if is_installed {
                info!("{}", message);
            } else {
                warn!("{}", message);
            }
        } else {
            info!("{} {}", mark, version.to_string());
        }
    }

    let (latest, _) = releases.latest();
    let using_latest = latest == &current;
    let mark = if using_latest {
        ", up to date <3"
    } else {
        ", wants upgrade!"
    };

    info!("");
    info!("{} version(s){}", total, mark);

    Ok(())
}
