use std::collections::HashMap;

use log::info;
use semver::VersionReq;

use crate::{
    binary, download, install::fetch, releases, runtime, version, Error, Result,
};

/// Attempt to upgrade to the latest version.
pub async fn update(name: &str, range: Option<VersionReq>) -> Result<()> {
    let version_file = version::file()?;
    let first_run = !version_file.exists();

    // Range filters not allowed on first run execution
    if first_run && range.is_some() {
        return Err(Error::RangeFilterNotAllowedOnFirstRun);
    }

    let mut names = vec![];
    names.extend_from_slice(&releases::INSTALL_EXE_NAMES);

    // Include shims on first run
    if first_run {
        names.extend_from_slice(&releases::INSTALL_SHIM_NAMES);
    }

    let version = fetch(
        name,
        names.as_slice(),
        true,
        true,
        None,
        range).await?;

    // Move over the shim executables
    if first_run {
        let version_dir = releases::dir(&version)?;
        let shims = releases::shim_map();
        let binaries = shims
            .into_iter()
            .map(|(s, _d)| {
                let path = version_dir.join(&s);
                (s, path) 
            })
            .collect::<HashMap<_, _>>();
        binary::rename(&binaries)?;
    }

    let message_kind = if first_run { "Installed" } else { "Updated" };
    info!("{} {}@{} ✓", message_kind, name, version.to_string());

    Ok(())
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
    binary::rename(&binaries)?;

    info!("Updated to {}@{} ✓", name, version.to_string());

    Ok(())
}
