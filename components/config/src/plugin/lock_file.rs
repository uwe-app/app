use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use semver::Version;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockFile {
    pub package: Vec<LockFileEntry>
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockFileEntry {
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    pub version: Version,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub source: Option<Url>,
    pub checksum: Option<String>,
    pub dependencies: Option<Vec<String>>,
}
