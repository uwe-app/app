use semver::Version;
use log::info;

use crate::Result;
use crate::updater;

#[derive(Debug)]
pub struct UpgradeOptions {}

pub fn upgrade(_options: UpgradeOptions) -> Result<()> {
    let (_, info) = updater::version()?;
    let installed_version = &info.version;

    let available = updater::load_remote_version()?;
    let current_version = available.version;

    let current = Version::parse(&current_version)?;
    let installed = Version::parse(installed_version)?;

    if current == installed {
        info!("Hypertext is up to date (v{})", current_version);
    } else {
        let (_name, _info, _bin, _bin_dir) = updater::update()?;
        let (_, info) = updater::version()?;
        info!("Upgraded to {}", info.version);
    }

    Ok(())
}
