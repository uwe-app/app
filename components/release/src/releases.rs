use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::Result;

pub static MANIFEST_JSON: &str = "manifest.json";

//pub static RUNTIME: &str = "runtime";
pub static RELEASES: &str = "releases";
pub static LATEST: &str = "latest";

pub static LINUX: &str = "linux";
pub static MACOS: &str = "macos";

pub static PUBLISH_EXE_NAMES: [&str; 5] = ["uwe", "upm", "uvm", "uwe-shim", "upm-shim"];
pub static INSTALL_EXE_NAMES: [&str; 2] = ["uwe", "upm"];
pub static VERSION_EXE_NAMES: [&str; 1] = ["uvm"];

#[cfg(target_os = "windows")]
pub static WINDOWS: &str = "windows";

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Releases {
    #[serde_as(as = "BTreeMap<DisplayFromStr, _>")]
    pub(crate) versions: BTreeMap<Version, ReleaseVersion>,
}

impl Releases {

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty() 
    }

    pub fn latest(&self) -> (&Version, &ReleaseVersion) {
        self.versions.iter().rev().take(1).next().unwrap()
    }

    pub fn contains(&self, version: &Version) -> bool {
        self.versions.contains_key(version)
    }

    pub fn filter(self, version: Option<VersionReq>) -> Self {
        if let Some(ref version) = version {
            let versions = self.versions
                .into_iter()
                .filter(|(v, _)| version.matches(&v))
                .collect::<BTreeMap<_, _>>();

            return Releases{ versions }
        }
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ReleaseVersion {
    #[serde(flatten)]
    pub(crate) platforms: HashMap<String, HashMap<String, String>>,
}

pub(crate) fn dir(version: &Version) -> Result<PathBuf> {
    Ok(dirs::releases_dir()?.join(version.to_string()))
}

pub(crate) fn exists(version: &Version) -> Result<bool> {
    Ok(dir(version)?.exists())
}

/// Load the release definition JSON.
pub(crate) fn load<P: AsRef<Path>>(target: P) -> Result<Releases> {
    let contents = fs::read_to_string(target.as_ref())?;
    let releases: Releases = serde_json::from_str(&contents)?;
    Ok(releases)
}

/// Load the releases list from the default runtime location.
pub fn mount() -> Result<Releases> {
    let releases_file = runtime_manifest_file()?;
    load(&releases_file)
}

/// Save the release definition JSON.
pub(crate) fn save<P: AsRef<Path>>(
    target: P,
    releases: &Releases,
) -> Result<()> {
    let contents = serde_json::to_vec_pretty(releases)?;
    let mut file = File::create(target.as_ref())?;
    file.write_all(contents.as_slice())?;
    Ok(())
}


/// Get the path to the local releases repository.
pub(crate) fn local_releases<P: AsRef<Path>>(
    base: P,
) -> Result<PathBuf> {
    Ok(base
        .as_ref()
        .join("..")
        .join(RELEASES)
        .canonicalize()?)
}

/// Get the release manifest file in the local releases repository.
pub(crate) fn local_manifest_file<P: AsRef<Path>>(
    base: P,
) -> Result<PathBuf> {
    Ok(local_releases(base)?.join(MANIFEST_JSON))
}

/// Get the release manifest file for the installed runtime used
/// for the install and upgrade processes.
pub fn runtime_manifest_file() -> Result<PathBuf> {
    Ok(dirs::releases_dir()?
        .join(MANIFEST_JSON)
        .canonicalize()?)
}

#[cfg(target_os = "windows")]
pub fn current_platform() -> String {
    WINDOWS.to_string()
}

#[cfg(target_os = "macos")]
pub fn current_platform() -> String {
    MACOS.to_string()
}

#[cfg(target_os = "linux")]
pub fn current_platform() -> String {
    LINUX.to_string()
}
