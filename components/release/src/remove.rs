use std::fs;

use log::info;
use semver::Version;

use crate::{releases, version, Error, Result};

/// Remove an installed version.
pub async fn remove(version: String) -> Result<()> {
    let version: Version = version
        .parse()
        .map_err(|_| Error::InvalidVersion(version))?;
    delete(&version).await
}

/// Remove versions older than the current version.
pub async fn prune() -> Result<()> {
    scm::system_repo::fetch_releases().await?;

    // Load the releases manifest
    let releases = releases::mount()?;

    // Get the current version
    let version_file = version::file()?;
    if !version_file.exists() {
        return Err(Error::NotInstalled);
    }

    let current = version::default_version()?;

    for (version, _) in releases.versions.iter() {
        if version.semver() < &current {
            if releases::exists(version.semver())? {
                delete(version.semver()).await?;
            }
        }
    }

    Ok(())
}

/// Delete a specific version.
async fn delete(version: &Version) -> Result<()> {
    let version_file = version::file()?;
    if version_file.exists() {
        let current = version::read(&version_file)?;
        if version == &current {
            return Err(Error::NoRemoveCurrent(version.to_string()));
        }
    }

    let version_dir = releases::dir(&version)?;
    if !version_dir.exists() {
        return Err(Error::VersionNotInstalled(
            version.to_string(),
            version_dir,
        ));
    }

    fs::remove_dir_all(&version_dir)?;
    info!("Deleted {}", version_dir.display());

    Ok(())
}
