use std::collections::HashMap;
use std::path::PathBuf;

use log::{info, warn};
use semver::VersionReq;

use crate::{
    binary, download, env, install::fetch, releases, version, Error, Result,
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

    if first_run {
        // Include shims on first run
        names.extend_from_slice(&releases::INSTALL_SHIM_NAMES);

        // Fetch plugin registry meta data
        scm::system_repo::fetch_registry().await?;
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

        welcome()?;
    }

    let current = version::default_version()?;
    if current == version {
        info!("Version {} is up to date ✓", version.to_string());
    } else {
        let message_kind = if first_run { "Installed" } else { "Updated" };
        info!("{} {}@{} ✓", message_kind, name, version.to_string());
    }

    Ok(())
}

pub async fn update_self(_current: &str) -> Result<()> {
    scm::system_repo::fetch_releases().await?;

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
