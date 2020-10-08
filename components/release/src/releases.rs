use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, BTreeMap};

use serde::{Serialize, Deserialize};
use serde_with::{serde_as, DisplayFromStr};
use semver::Version;

use crate::Result;

static RELEASES_JSON: &str = "releases.json";

pub static RUNTIME: &str = "runtime";
pub static RELEASE: &str = "release";
pub static LATEST: &str = "latest";

pub static LINUX: &str = "linux";
pub static MACOS: &str = "macos";
pub static WINDOWS: &str = "windows";

pub static PUBLISH_EXE_NAMES: [&str; 3] = ["uwe", "upm", "uvm"];
pub static INSTALL_EXE_NAMES: [&str; 2] = ["uwe", "upm"];

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Releases {
    #[serde_as(as = "BTreeMap<DisplayFromStr, _>")]
    pub(crate) versions: BTreeMap<Version, ReleaseVersion>,
}

impl Releases {
    pub fn latest(&self) -> (&Version, &ReleaseVersion) {
        self.versions.iter().rev().take(1).next().unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ReleaseVersion {
    #[serde(flatten)]
    pub(crate) platforms: HashMap<String, HashMap<String, String>>
}

pub(crate) fn dir(version: &Version) -> Result<PathBuf>{
    Ok(cache::get_runtime_dir()?
        .join(RELEASE).join(version.to_string()))
}

/// Load the release definition JSON.
pub(crate) fn load<P: AsRef<Path>>(target: P) -> Result<Releases> {
    let contents = fs::read_to_string(target.as_ref())?;
    let releases: Releases = serde_json::from_str(&contents)?;
    Ok(releases)
}

/// Save the release definition JSON.
pub(crate) fn save<P: AsRef<Path>>(target: P, releases: &Releases) -> Result<()> {
    let contents = serde_json::to_vec_pretty(releases)?;
    let mut file = File::create(target.as_ref())?;
    file.write_all(contents.as_slice())?;
    Ok(())
}

/// Get the release manifest file for a local relative repository 
/// used during the publish process.
pub(crate) fn repo_manifest_file<P: AsRef<Path>>(manifest: P) -> Result<PathBuf> {
    Ok(manifest.as_ref()
        .join("..")
        .join(RUNTIME)
        .join(RELEASES_JSON)
        .canonicalize()?)
}

/// Get the release manifest file for the installed runtime used 
/// for the install and upgrade processes.
pub(crate) fn runtime_manifest_file() -> Result<PathBuf> {
    Ok(cache::get_runtime_dir()?
        .join(RELEASES_JSON)
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
