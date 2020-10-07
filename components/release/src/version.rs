use std::path::PathBuf;
use std::fs::{self, File};
use std::io::Write;

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use semver::Version;

use crate::Result;

static VERSION_FILE: &str = "version.toml";

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionInfo {
    #[serde_as(as = "DisplayFromStr")]
    version: Version,
}

fn get_version_file() -> Result<PathBuf> {
    Ok(cache::get_release_dir()?.join(VERSION_FILE))
}

/// Write out the version file.
pub(crate) fn write(version: &Version) -> Result<()> {
    let version_file = get_version_file()?;
    let info = VersionInfo {version: version.clone()};
    let mut file = File::create(version_file)?;
    let content = toml::to_string(&info)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Read the version file.
pub(crate) fn read() -> Result<(PathBuf, VersionInfo)> {
    let version_file = get_version_file()?;
    let content = fs::read_to_string(&version_file)?;
    let info: VersionInfo = toml::from_str(&content)?;
    Ok((version_file, info))
}
