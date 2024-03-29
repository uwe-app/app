use log::debug;

use semver::Version;

use crate::{checksum, releases, Error, Result};

use config::plugin::VersionKey;

/// Verify the checksums for a version.
pub(crate) fn test(
    version: &Version,
    names: &[&str],
) -> Result<(bool, String, String)> {
    // Load the releases manifest.
    let releases = releases::mount()?;

    // Check the version information exists
    let version_dir = releases::dir(version)?;
    if !version_dir.exists() {
        return Err(Error::VersionNotInstalled(
            version.to_string(),
            version_dir,
        ));
    }

    let release_version = VersionKey::from(version);

    let info = releases
        .versions
        .get(&release_version)
        .ok_or_else(|| Error::VersionNotFound(version.to_string()))?;

    let checksums = info.platforms.get(&releases::current_platform()).unwrap();

    for (name, expected) in checksums {
        if !names.contains(&name.as_str()) {
            continue;
        }

        let file_path = version_dir.join(name);
        if !file_path.exists() || !file_path.is_file() {
            return Ok((false, name.to_string(), expected.to_string()));
        }

        debug!("Verify {} ({})", name, expected);

        let received = hex::encode(checksum::digest(&file_path)?);
        if received != expected.hex() {
            return Ok((false, name.to_string(), expected.hex()));
        }
    }

    Ok((true, String::new(), String::new()))
}
