use std::path::PathBuf;

use log::{info, warn};

use cache::CacheComponent;

use crate::{Result, releases, env, download, binary};

fn finish() -> Result<PathBuf> {
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

/// Install the application components.
pub async fn install(name: String) -> Result<()> {
    // Ensure we have the runtime assets so we can
    // access the release definitions
    let components = vec![CacheComponent::Runtime];
    cache::update(components)?;

    // Load the releases manifest.
    let releases_file = releases::runtime_manifest_file()?;
    let releases = releases::load(&releases_file)?;

    // Get the latest available version.
    let (version, info) = releases.latest();

    // Download all the artifacts for the version.
    let binaries = download::all(version, info).await?;
    binary::permissions(&binaries)?;
    binary::symlink(&binaries)?;

    finish()?;

    info!("Installed {}@{} ✓", name, version.to_string());

    Ok(())
}
