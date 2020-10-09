use semver::Version;

use crate::{Result, releases, runtime, info};

pub async fn update(current: String) -> Result<()> {
    // Must have latest runtime assets
    runtime::fetch().await?;

    // This is the version of the current executing program
    let version: Version = current.parse()?;

    // Load the releases manifest.
    let releases_file = releases::runtime_manifest_file()?;
    let releases = releases::load(&releases_file)?;

    // Get the latest available version.
    let (installed, _) = releases.latest();

    if &version == installed {
        return info::upto_date(&version);
    }

    Ok(())
}
