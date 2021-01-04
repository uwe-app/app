use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use semver::Version;

use crate::{Error, Result};

static VERSION_FILE: &str = ".uwe-version";

pub(crate) fn file() -> Result<PathBuf> {
    Ok(dirs::root_dir()?.join(VERSION_FILE))
}

/// Write out the version file.
pub(crate) fn write<P: AsRef<Path>>(path: P, version: &Version) -> Result<()> {
    let mut file = File::create(path.as_ref())?;
    file.write_all(version.to_string().as_bytes())?;
    Ok(())
}

/// Read the version file.
pub(crate) fn read<P: AsRef<Path>>(path: P) -> Result<Version> {
    let content = fs::read_to_string(path.as_ref())?;
    let info: Version = content.parse()?;
    Ok(info)
}

pub fn default_version() -> Result<Version> {
    Ok(read(file()?)?)
}

/// Find and parse a version from the local version file
/// searching parent directories until the root.
pub fn find_local_version<P: AsRef<Path>>(path: P) -> Result<(Option<Version>, Option<PathBuf>)> {
    let mut dir = path.as_ref();
    let mut version_file = dir.join(VERSION_FILE);
    while !version_file.exists() {
        if let Some(parent) = dir.parent() {
            version_file = parent.join(VERSION_FILE);
            dir = parent;
        } else { break }
    }

    if version_file.exists() && version_file.is_file() {
        let content = fs::read_to_string(&version_file)?; 
        let version: Version = content.parse::<Version>().map_err(
            |e| Error::VersionFileRead(version_file.clone(), e.to_string()))?;
        return Ok((Some(version), Some(version_file)))
    }

    Ok((None, None))
}

