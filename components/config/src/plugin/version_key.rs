use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use semver::Version;

/// Wrapper for the version so we order releases in the
/// reverse direction with the latest as the first element.
///
/// This is required because the generated manifest is used
/// as a collections data source for the releases.uwe.app
/// website and we want to show the most recent release first.
#[derive(Debug, Clone, Eq)]
pub struct VersionKey {
    version: Version,
}

impl FromStr for VersionKey {
    type Err = crate::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let version: Version = s.parse()?;
        Ok(VersionKey { version })
    }
}

impl VersionKey {
    pub fn semver(&self) -> &Version {
        &self.version
    }
}

impl fmt::Display for VersionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)
    }
}

impl From<&Version> for VersionKey {
    fn from(version: &Version) -> Self {
        Self {
            version: version.clone(),
        }
    }
}

impl From<Version> for VersionKey {
    fn from(version: Version) -> Self {
        Self { version }
    }
}

impl Into<Version> for VersionKey {
    fn into(self) -> Version {
        self.version
    }
}

/// Invert the ordering for versions.
impl Ord for VersionKey {
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

impl PartialOrd for VersionKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for VersionKey {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}
