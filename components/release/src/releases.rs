use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::fmt;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use serde_with::{serde_as, DisplayFromStr};
use config::plugin::VersionKey;

use crate::Result;

pub static MANIFEST_JSON: &str = "manifest.json";

pub static RELEASES: &str = "releases";
pub static LATEST: &str = "latest";

pub static LINUX: &str = "linux";
pub static MACOS: &str = "macos";
#[cfg(target_os = "windows")]
pub static WINDOWS: &str = "windows";

pub static PUBLISH_EXE_NAMES: [&str; 5] = ["uwe", "upm", "uvm", "uwe-shim", "upm-shim"];
pub static INSTALL_EXE_NAMES: [&str; 2] = ["uwe", "upm"];
pub static VERSION_EXE_NAMES: [&str; 3] = ["uvm", "uwe-shim", "upm-shim"];
pub static INSTALL_SHIM_NAMES: [&str; 2] = ["uwe-shim", "upm-shim"];

pub static SHIM: [(&str, &str); 2] = [("uwe-shim", "uwe"), ("upm-shim", "upm")];

pub(crate) type Platform = String;
pub(crate) type ExecutableTargets =
    BTreeMap<Platform, BTreeMap<String, ExecutableArtifact>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutableArtifact {
    #[serde(skip)]
    pub(crate) path: PathBuf,
    #[serde(serialize_with = "to_hex", deserialize_with = "from_hex")]
    pub(crate) digest: Vec<u8>,
    pub(crate) size: u64,
}

fn to_hex<S>(digest: &Vec<u8>, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where S: Serializer
{
    serializer.serialize_str(&hex::encode(digest))
}

fn from_hex<'de, D>(deserializer: D) -> std::result::Result<Vec<u8>, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| {
            hex::decode(&string)
                .map_err(|err| Error::custom(err.to_string()))
        })
}

impl ExecutableArtifact {
    pub fn hex(&self) -> String {
        hex::encode(&self.digest)
    }
}

impl fmt::Display for ExecutableArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hex())
    }
}

pub fn shim_map() -> HashMap<String, String> {
    SHIM
        .iter()
        .map(|(s, e)| {
            (s.to_string(), e.to_string()) 
        })
        .collect::<HashMap<_, _>>()
}

#[derive(Debug, Clone)]
pub struct Releases {
    pub(crate) versions: BTreeMap<VersionKey, ReleaseInfo>,
}

impl From<ReleaseManifest> for Releases {
    fn from(manifest: ReleaseManifest) -> Self {
        let mut versions = BTreeMap::new();
        if let Some(mut info) = manifest.latest {
            let semver = info.version.take().unwrap();
            versions.insert(VersionKey::from(semver), info);
        }
        manifest.versions
            .into_iter()
            .for_each(|mut info| {
                let semver = info.version.take().unwrap();
                versions.insert(VersionKey::from(semver), info);
            });
        Releases {versions}
    }
}

impl Releases {

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty() 
    }

    pub fn latest(&self) -> (&Version, &ReleaseInfo) {
        let (k, v) = self.versions.iter().take(1).next().unwrap();
        (k.semver(), v)
    }

    pub fn contains(&self, version: &Version) -> bool {
        let release_version = VersionKey::from(version);
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
pub struct ReleaseManifest {
    latest: Option<ReleaseInfo>,
    versions: Vec<ReleaseInfo>,
}

impl From<&Releases> for ReleaseManifest {
    fn from(releases: &Releases) -> Self {
        let releases = releases.clone();
        let mut manifest: ReleaseManifest = Default::default();
        let mut it = releases.versions.into_iter();

        if let Some((release_version, mut release_info)) = it.next() {
            release_info.version = Some(release_version.into());
            manifest.latest = Some(release_info); 
        }

        while let Some((release_version, mut release_info)) = it.next() {
            release_info.version = Some(release_version.into());
            manifest.versions.push(release_info); 
        }

        manifest
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ReleaseInfo {
    #[serde_as(as = "Option<DisplayFromStr>")]
    version: Option<Version>,
    #[serde(flatten)]
    pub(crate) platforms: ExecutableTargets,
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
    let manifest: ReleaseManifest = serde_json::from_str(&contents)?;
    let releases: Releases = manifest.into();
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
    let manifest: ReleaseManifest = releases.into();
    let mut file = File::create(target.as_ref())?;
    serde_json::to_writer_pretty(&mut file, &manifest)?;
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
