use std::fs;

use semver::Version;
use log::info;

use crate::{Error, Result, releases, version};

/// Remove an installed version.
pub async fn remove(name: String, version: String) -> Result<()> {
    let semver: Version = version.parse()
        .map_err(|_| Error::InvalidVersion(version))?;

    let version_file = version::file()?;
    if version_file.exists() {
        let version_info = version::read(&version_file)?; 
        if semver == version_info.version {
            return Err(
                Error::NoRemoveCurrent(semver.to_string()));
        }
    }

    let version_dir = releases::dir(&semver)?;
    if !version_dir.exists() {
        return Err(
            Error::VersionNotInstalled(
                semver.to_string(), version_dir));
    }

    fs::remove_dir_all(&version_dir)?;
    info!("Deleted {}", version_dir.display());

    Ok(())
}
