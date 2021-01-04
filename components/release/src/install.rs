use log::{info, warn};
use semver::{Version, VersionReq};

use crate::{
    binary, download, info, releases, runtime, verify, version, Error,
    Result,
};

/// Install a version and select it so it is the current version.
pub async fn select(name: &str, version: String) -> Result<()> {
    let version: Version = version
        .parse()
        .map_err(|_| Error::InvalidVersion(version))?;

    fetch(
        name,
        &releases::INSTALL_EXE_NAMES,
        true,
        false,
        Some(version.clone()),
        None).await?;

    info!("Installed {}@{} ✓", name, version.to_string());
    Ok(())
}

/// Install a version but do not select it.
pub async fn install(name: &str, version: String) -> Result<()> {
    let version: Version = version
        .parse()
        .map_err(|_| Error::InvalidVersion(version))?;

    fetch(
        name,
        &releases::INSTALL_EXE_NAMES,
        false,
        false,
        Some(version.clone()),
        None).await?;

    info!("Installed {}@{} ✓", name, version.to_string());

    Ok(())
}

/// Install the application components.
pub(crate) async fn fetch(
    name: &str,
    names: &[&str],
    select: bool,
    latest: bool,
    version: Option<Version>,
    range: Option<VersionReq>,
) -> Result<Version> {
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
        let current = version::default_version()?;
        if &current == version {
            info::upto_date(&version)?;
            return Ok(version.clone())
        }
    }

    //let names = &releases::INSTALL_EXE_NAMES;
    //names.foo();

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
            return Ok(version.clone());
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

    if select {
        version::write(&version_file, version)?;
    }

    Ok(version.clone())
}
