use log::info;
use semver::Version;

use updater;

use crate::Result;

pub fn try_upgrade(runtime: bool) -> Result<()> {

    let prefs = preference::load()?;
    cache::update(&prefs, vec![cache::CacheComponent::Runtime])?;

    // Only upgrading the runtime assets
    if runtime {
        return Ok(())
    }

    let (_, info) = updater::version()?;
    let installed_version = &info.version;

    let available = updater::load_remote_version()?;
    let current_version = available.version;

    let current = Version::parse(&current_version)?;
    let installed = Version::parse(installed_version)?;

    if current == installed {
        info!("Hypertext is up to date (v{})", current_version);
    } else {
        let (_name, info, _bin, _bin_dir) = updater::update()?;
        info!("Upgraded to {}", info.version);
    }

    Ok(())
}
