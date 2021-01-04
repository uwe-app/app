use log::info;
use semver::VersionReq;

use crate::{
    binary, download, install::fetch, releases, runtime, Result,
};

/// Attempt to upgrade to the latest version.
pub async fn update(name: &str, range: Option<VersionReq>) -> Result<()> {
    fetch(name, true, true, None, range).await
}

pub async fn update_self(_current: &str) -> Result<()> {
    runtime::fetch_releases().await?;

    let exe = std::env::current_exe()?;
    let name = exe.file_name().unwrap().to_string_lossy().to_owned();

    // Load the releases manifest.
    let releases = releases::mount()?;

    // Get the latest available version.
    let (version, info) = releases.latest();

    /*
    // This is the version of the current executing program
    let current: Version = current.parse()?;

    if &current == version {
        return info::upto_date(&current);
    }
    */

    // Download the uvm artifact for the version.
    let binaries =
        download::all(version, info, &releases::VERSION_EXE_NAMES).await?;
    binary::permissions(&binaries)?;
    binary::copy(&binaries)?;

    info!("Updated to {}@{} ✓", name, version.to_string());

    Ok(())
}
