use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::cmp::Ordering;
use std::str::FromStr;
use std::fmt;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::Result;

pub static MANIFEST_JSON: &str = "manifest.json";

pub static RELEASES: &str = "releases";
pub static LATEST: &str = "latest";

pub static LINUX: &str = "linux";
pub static MACOS: &str = "macos";

pub static PUBLISH_EXE_NAMES: [&str; 5] = ["uwe", "upm", "uvm", "uwe-shim", "upm-shim"];
pub static INSTALL_EXE_NAMES: [&str; 2] = ["uwe", "upm"];
pub static VERSION_EXE_NAMES: [&str; 3] = ["uvm", "uwe-shim", "upm-shim"];
pub static INSTALL_SHIM_NAMES: [&str; 2] = ["uwe-shim", "upm-shim"];

pub static SHIM: [(&str, &str); 2] = [("uwe-shim", "uwe"), ("upm-shim", "upm")];

#[cfg(target_os = "windows")]
pub static WINDOWS: &str = "windows";

pub(crate) type Platform = String;
pub(crate) type ExecutableName = String;
pub(crate) type Checksum = String;
pub(crate) type ExecutableTargets =
    BTreeMap<Platform, BTreeMap<ExecutableName, ExecutableArtifact>>;

#[derive(Debug)]
pub struct ExecutableArtifact {
    pub(crate) path: PathBuf,
    pub(crate) digest: Vec<u8>,
}

pub fn shim_map() -> HashMap<String, String> {
    SHIM
        .iter()
        .map(|(s, e)| {
            (s.to_string(), e.to_string()) 
        })
        .collect::<HashMap<_, _>>()
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Releases {
    #[serde_as(as = "BTreeMap<DisplayFromStr,_>")]
    pub(crate) versions: BTreeMap<ReleaseVersion, ReleaseInfo>,
}

/// Wrapper for the version so we order releases in the 
/// reverse direction with the latest as the first element.
///
/// This is required because the generated manifest is used 
/// as a collections data source for the releases.uwe.app 
/// website and we want to show the most recent release first.
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct ReleaseVersion {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(flatten)]
    version: Version,
}

impl FromStr for ReleaseVersion {
    type Err = crate::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let version: Version = s.parse()?;
        Ok(ReleaseVersion{ version })
    }
}

impl ReleaseVersion {
    pub fn semver(&self) -> &Version {
        &self.version 
    }
}

impl fmt::Display for ReleaseVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)
    }
}

impl From<&Version> for ReleaseVersion {
    fn from(version: &Version) -> Self {
        Self {version: version.clone()}
    }
}

impl From<Version> for ReleaseVersion {
    fn from(version: Version) -> Self {
        Self {version}
    }
}

impl Into<Version> for ReleaseVersion {
    fn into(self) -> Version {
        self.version
    }
}

/// Invert the ordering for versions.
impl Ord for ReleaseVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.version == other.version {
            Ordering::Equal
        } else if self.version < other.version {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

impl PartialOrd for ReleaseVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ReleaseVersion {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl Releases {

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty() 
    }

    pub fn latest(&self) -> (&Version, &ReleaseInfo) {
        //self.versions.iter().rev().take(1).next().unwrap()
        let (k, v) = self.versions.iter().take(1).next().unwrap();
        (k.semver(), v)
    }

    pub fn contains(&self, version: &Version) -> bool {
        let release_version = ReleaseVersion::from(version);
        self.versions.contains_key(&release_version)
    }

    pub fn filter(self, version: Option<VersionReq>) -> Self {
        if let Some(ref version) = version {
            let versions = self.versions
                .into_iter()
                .filter(|(v, _)| version.matches(v.semver()))
                .collect::<BTreeMap<_, _>>();

            return Releases{ versions }
        }
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ReleaseInfo {
    #[serde(flatten)]
    pub(crate) platforms: HashMap<Platform, HashMap<ExecutableName, Checksum>>,
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
    let contents = serde_json::to_vec_pretty(&releases)?;
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
