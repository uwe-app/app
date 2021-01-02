use log::info;
use semver::Version;

use crate::{binary, download, info, releases, runtime, Result, install::fetch};

/// Attempt to upgrade to the latest version.
pub async fn update(name: &str) -> Result<()> {
    fetch(name, true, true, None).await
}

pub async fn update_self(current: &str) -> Result<()> {
    // Must have latest runtime assets
    runtime::fetch().await?;

    let exe = std::env::current_exe()?;
    let name = exe.file_name().unwrap().to_string_lossy().to_owned();

    // This is the version of the current executing program
    let current: Version = current.parse()?;

    // Load the releases manifest.
    let releases_file = releases::runtime_manifest_file()?;
    let releases = releases::load(&releases_file)?;

    // Get the latest available version.
    let (version, info) = releases.latest();
    if &current == version {
        return info::upto_date(&current);
    }

    // Download the uvm artifact for the version.
    let binaries =
        download::all(version, info, &releases::VERSION_EXE_NAMES).await?;
    binary::permissions(&binaries)?;
    binary::symlink(&binaries)?;

    info!("Updated to {}@{} ✓", name, version.to_string());

    Ok(())
}
