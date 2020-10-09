use std::path::PathBuf;

use log::{info, warn};
use semver::Version;

use crate::{
    Error,
    Result,
    binary,
    download,
    env,
    info,
    releases,
    runtime,
    version,
};

fn welcome() -> Result<PathBuf> {
    let bin_dir = cache::get_bin_dir()?;

    // Write out the env file
    env::write(&bin_dir)?;

    // Try to configure the shell paths
    let (shell_ok, shell_write, shell_name, shell_file) =
        env::update_shell_profile()?;
    if shell_ok {
        if shell_write {
            info!("");
            info!("Updated {} at {}", shell_name, shell_file.display());
        }
    } else {
        warn!("");
        warn!("Update your PATH to include {}", bin_dir.display());
    }

    let source_path = env::get_source_env().trim().to_string();

    info!("");
    info!("To update your current shell session run:");
    info!("");
    info!("   {}", source_path);
    info!("");

    Ok(bin_dir)
}

/// Attempt to upgrade to the latest version.
pub async fn latest(name: String) -> Result<()> {
    fetch(name, true, true, None).await
}

/// Install a version and select it so it is the current version.
pub async fn select(name: String, version: String) -> Result<()> {
    let semver: Version = version
        .parse()
        .map_err(|_| Error::InvalidVersion(version))?;
    fetch(name, true, false, Some(semver)).await
}

/// Install a version but do not select it.
pub async fn install(name: String, version: String) -> Result<()> {
    let semver: Version = version
        .parse()
        .map_err(|_| Error::InvalidVersion(version))?;
    fetch(name, false, false, Some(semver)).await
}

/// Install the application components.
async fn fetch(
    name: String,
    select: bool,
    latest: bool,
    version: Option<Version>,
) -> Result<()> {
    // Must have latest runtime assets
    runtime::fetch().await?;

    // Load the releases manifest.
    let releases_file = releases::runtime_manifest_file()?;
    let releases = releases::load(&releases_file)?;

    let (version, info) = if let Some(ref request) = version {
        let info = releases
            .versions
            .get(request)
            .ok_or_else(|| Error::VersionNotFound(request.to_string()))?;
        (request, info)
    } else {
        // Get the latest available version.
        releases.latest()
    };

    let version_file = version::file()?;

    // If we want the latest version and currently are the latest
    // version then no need to proceed
    if latest && version_file.exists() {
        let info = version::read(&version_file)?;
        if &info.version == version {
            return info::upto_date(&version);
        }
    }

    // Download all the artifacts for the version.
    let binaries = download::all(version, info).await?;
    binary::permissions(&binaries)?;

    if select {
        binary::symlink(&binaries)?;
    }

    let first_run = !version_file.exists();
    if first_run {
        welcome()?;
    }

    if select {
        version::write(&version_file, version)?;
    }

    info!("Installed {}@{} ✓", name, version.to_string());

    Ok(())
}
