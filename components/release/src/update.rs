use semver::Version;
use log::info;

use crate::{
    Result,
    releases,
    runtime,
    binary,
    info,
    download,
};

pub async fn update(current: &str) -> Result<()> {
    // Must have latest runtime assets
    runtime::fetch().await?;

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
    let binaries = download::all(
        version, info, &releases::VERSION_EXE_NAMES).await?;
    binary::permissions(&binaries)?;
    binary::symlink(&binaries)?;

    info!("Updated to uvm@{} ✓", version.to_string());

    Ok(())
}
