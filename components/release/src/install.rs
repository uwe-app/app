use std::path::PathBuf;

use log::{info, warn};
use semver::{Version, VersionReq};

use crate::{
    binary, download, env, info, releases, runtime, verify, version, Error,
    Result,
};

fn welcome() -> Result<PathBuf> {
    let bin_dir = dirs::bin_dir()?;

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

/// Install a version and select it so it is the current version.
pub async fn select(name: &str, version: String) -> Result<()> {
    let semver: Version = version
        .parse()
        .map_err(|_| Error::InvalidVersion(version))?;
    fetch(name, true, false, Some(semver), None).await
}

/// Install a version but do not select it.
pub async fn install(name: &str, version: String) -> Result<()> {
    let semver: Version = version
        .parse()
        .map_err(|_| Error::InvalidVersion(version))?;
    fetch(name, false, false, Some(semver), None).await
}

/// Install the application components.
pub(crate) async fn fetch(
    name: &str,
    select: bool,
    latest: bool,
    version: Option<Version>,
    range: Option<VersionReq>,
) -> Result<()> {
    // Must update the cache of releases
    runtime::fetch_releases().await?;

    let registry_dir = dirs::registry_dir()?;
    if !registry_dir.exists() {
        // Fetch plugin registry meta data
        runtime::fetch_registry().await?;
    }

    let runtime_dir = dirs::runtime_dir()?;
    if !runtime_dir.exists() {
        // Fetch runtime assets if they don't exist
        runtime::fetch().await?;
    }

    // Load the releases manifest.
    let releases = releases::mount()?.filter(range);
    if releases.is_empty() {
        return Err(Error::NoReleasesFound) 
    }

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

    let names = &releases::INSTALL_EXE_NAMES;

    if releases::exists(version)? {
        let version_dir = releases::dir(version)?;
        info!("Verify {}", version_dir.display());
        let (verified, exe_name, _) = verify::test(version, names)?;
        if verified {
            if select {
                //binary::symlink_names(&version_dir, names)?;
                version::write(&version_file, version)?;
            }

            info!("Installation {}@{} is ok ✓", name, version.to_string());
            return Ok(());
        } else {
            warn!(
                "Existing installation for {}@{} may be corrupt",
                exe_name, version
            );
        }
    }

    // Download all the artifacts for the version.
    let binaries = download::all(version, info, names).await?;
    binary::permissions(&binaries)?;

    /*
    if select {
        binary::symlink(&binaries)?;
    }
    */

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
