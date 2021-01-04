use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use semver::Version;

use crate::{Result};

static VERSION_FILE: &str = "version.toml";

static LOCAL_VERSION_FILE: &str = ".uwe-version";


#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionInfo {
    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,
}

pub(crate) fn file() -> Result<PathBuf> {
    Ok(dirs::releases_dir()?
        .join(VERSION_FILE))
}

/// Write out the version file.
pub(crate) fn write<P: AsRef<Path>>(path: P, version: &Version) -> Result<()> {
    let info = VersionInfo {
        version: version.clone(),
    };
    let mut file = File::create(path.as_ref())?;
    let content = toml::to_string(&info)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Read the version file.
pub(crate) fn read<P: AsRef<Path>>(path: P) -> Result<VersionInfo> {
    let content = fs::read_to_string(path.as_ref())?;
    let info: VersionInfo = toml::from_str(&content)?;
    Ok(info)
}

pub fn default_version() -> Result<Version> {
    let info = read(file()?)?;
    Ok(info.version)
}

/// Find and parse a version from the local version file
/// searching parent directories until the root.
pub fn find_local_version<P: AsRef<Path>>(path: P) -> Result<(Option<Version>, Option<PathBuf>)> {
    let mut dir = path.as_ref();
    let mut version_file = dir.join(LOCAL_VERSION_FILE);
    while !version_file.exists() {
        if let Some(parent) = dir.parent() {
            version_file = parent.join(LOCAL_VERSION_FILE);
            dir = parent;
        } else { break }
    }

    if version_file.exists() && version_file.is_file() {
        let content = fs::read_to_string(&version_file)?; 
        let version: Version = content.parse()?;
        return Ok((Some(version), Some(version_file)))
    }

    Ok((None, None))
}

