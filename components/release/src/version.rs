use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use semver::Version;

use crate::{releases, Result};

static VERSION_FILE: &str = "version.toml";

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionInfo {
    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,
}

pub(crate) fn file() -> Result<PathBuf> {
    Ok(dirs::get_runtime_dir()?
        .join(releases::RELEASE)
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
