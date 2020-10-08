use log::info;

use crate::{Error, Result, releases, version};

/// List versions.
pub async fn list() -> Result<()> {

    // Load the releases manifest
    let releases_file = releases::runtime_manifest_file()?;
    let releases = releases::load(&releases_file)?;

    // Get the current version
    let version_file = version::file()?;
    if !version_file.exists() {
        return Err(Error::NotInstalled)
    }

    let info = version::read(&version_file)?;
    let current = &info.version;

    let total = releases.versions.iter().count();

    info!("-------------------------------");
    info!("| ◯ (installed) | ✓ (current) |");
    info!("-------------------------------");
    info!("");

    for (version, _) in releases.versions.iter() {
        let version_dir = cache::get_runtime_dir()?
            .join(releases::RELEASE).join(version.to_string());
        let is_installed = version_dir.exists() && version_dir.is_dir();
        let mark = if is_installed {"◯"} else {"-"};
        if current == version {
            info!("{} {} ✓", mark, version.to_string());
        } else {
            info!("{} {}", mark, version.to_string());
        }
    }

    let (latest, _) = releases.latest();
    let using_latest = latest == current;
    let mark = if using_latest {", up to date <3"} else {", needs update!"};

    info!("");
    info!("{} version(s){}", total, mark);

    Ok(())
}
