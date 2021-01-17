use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use log::{debug, info, warn};
use semver::{Version, VersionReq};

use crate::{download, env, releases, verify, version, Error, Result};

use config::plugin::VersionKey;

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
        None,
    )
    .await?;

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
        None,
    )
    .await?;

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
    scm::system_repo::fetch_releases().await?;
    info!("Downloaded releases ✓");

    // Load the releases manifest.
    let releases = releases::mount()?.filter(range);
    if releases.is_empty() {
        return Err(Error::NoReleasesFound);
    }

    let (version, info) = if let Some(ref request) = version {
        let info = releases
            .versions
            .get(&VersionKey::from(request))
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
            return Ok(version.clone());
        }
    }

    if releases::exists(version)? {
        let version_dir = releases::dir(version)?;
        info!("Verify {}", version_dir.display());
        let (verified, exe_name, _) = verify::test(version, names)?;
        if verified {
            if select {
                //binary::symlink_names(&version_dir, names)?;
                version::write(&version_file, version)?;
            }
            info!("Version {}@{} is installed and ok ✓", name, version.to_string());
            return Ok(version.clone());
        }
    }

    // Download all the artifacts for the version.
    info!("Download {} components: {}", names.len(), names.join(", "));
    let binaries = download::all(version, info, names).await?;
    utils::terminal::clear_previous_line()?;
    info!("Downloaded binary components ✓");

    permissions(&binaries)?;

    if select {
        version::write(&version_file, version)?;
    }

    info!("Download documentation...");
    plugin::install_docs(Some(version)).await?;
    utils::terminal::clear_current_line()?;
    utils::terminal::clear_previous_line()?;
    info!("Downloaded documentation ✓");

    Ok(version.clone())
}

#[cfg(target_os = "windows")]
pub(crate) fn permissions(binaries: &HashMap<String, PathBuf>) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(crate) fn permissions(binaries: &HashMap<String, PathBuf>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for (_name, src) in binaries {
        let metadata = src.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&src, permissions)?;
    }
    Ok(())
}

pub(crate) fn rename(binaries: &HashMap<String, PathBuf>) -> Result<()> {
    let releases_dir = dirs::releases_dir()?;
    let bin_dir = dirs::bin_dir()?;

    let shims = releases::shim_map();
    for (name, src) in binaries {
        let bin_name = if let Some(shim_dest) = shims.get(name) {
            shim_dest.to_string()
        } else {
            name.to_string()
        };

        let dest = bin_dir.join(&bin_name);
        if dest.exists() {
            fs::remove_file(&dest)?;
        }

        let short_src = src.strip_prefix(&releases_dir)?;
        debug!("Move {} -> {}", short_src.display(), dest.display());
        std::fs::rename(src, dest)?;
    }
    Ok(())
}

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
    let root_dir = dirs::root_dir()?;
    let first_run = !version_file.exists();

    // Range filters not allowed on first run execution
    if first_run && range.is_some() {
        return Err(Error::RangeFilterNotAllowedOnFirstRun);
    }

    let mut names = vec![];
    names.extend_from_slice(&releases::INSTALL_EXE_NAMES);

    if first_run {
        // Create the root installation directory
        if !root_dir.exists() {
            std::fs::create_dir(&root_dir)?;
        }

        // Include shims on first run
        names.extend_from_slice(&releases::INSTALL_SHIM_NAMES);

        // Fetch plugin registry meta data
        scm::system_repo::fetch_registry().await?;
        info!("Downloaded plugin registry ✓");
    }

    let mut current = version::default_version().ok();

    let version =
        fetch(name, names.as_slice(), true, true, None, range).await?;

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
        rename(&binaries)?;

        welcome()?;
    }

    if let Some(current) = current.take() {
        if current == version {
            info!("Version {} is up to date ✓", version);
        } else {
            show_message(first_run, name, version);
        }
    } else {
        show_message(first_run, name, version);
    }

    Ok(())
}

fn show_message(first_run: bool, name: &str, version: Version) {
    let message_kind = if first_run { "Installed" } else { "Updated" };
    info!("{} {}@{} ✓", message_kind, name, version);
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
    permissions(&binaries)?;
    rename(&binaries)?;

    info!("Updated to {}@{} ✓", name, version.to_string());

    Ok(())
}
