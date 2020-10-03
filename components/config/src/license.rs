use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum LicenseGroup {
    One(License),
    Many(Vec<License>),
}

impl LicenseGroup {
    pub fn to_vec(&self) -> Vec<&License> {
        match *self {
            LicenseGroup::One(ref license) => {
                vec![license]
            }
            LicenseGroup::Many(ref licenses) => {
                licenses.iter().collect::<Vec<_>>()
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum License {
    Spdx(String),
    // NOTE: later we may support non-spdx license declarations
}

impl fmt::Display for License {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            License::Spdx(ref value) => {
                write!(f, "{}", value)
            }
        }
    }
}
