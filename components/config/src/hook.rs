use std::collections::{hash_set, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{profile::ProfileName, utils::href::UrlPath, Error, Result};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HookMap {
    #[serde(rename = "run")]
    exec: HashSet<HookConfig>,
}

impl HookMap {
    pub fn exec(&self) -> &HashSet<HookConfig> {
        &self.exec
    }

    pub fn iter(&self) -> hash_set::Iter<'_, HookConfig> {
        self.exec.iter()
    }

    pub fn prepare(&mut self, base: &PathBuf) -> Result<()> {
        let mut out: HashSet<HookConfig> = HashSet::new();
        for mut v in self.exec.drain() {
            if v.path.is_empty() {
                return Err(Error::HookPathEmpty(base.to_path_buf()));
            }
            v.base = base.to_path_buf();

            out.insert(v);
        }

        self.exec = out;
        Ok(())
    }

    pub fn append(&mut self, other: &mut HookMap) {
        for v in other.exec.drain() {
            self.exec.insert(v);
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(default)]
pub struct HookConfig {
    // Command path.
    pub path: String,

    // Command arguments.
    pub args: Option<Vec<String>>,

    pub stdout: Option<bool>,
    pub stderr: Option<bool>,

    // Marks the hook to run after a build
    pub after: Option<bool>,

    // Only run for these profiles
    pub profiles: Option<Vec<ProfileName>>,

    // List of files expected by this hook
    pub files: Option<Vec<UrlPath>>,

    // Whether to trigger the hook when the
    // expected files changes.
    pub watch: Option<bool>,

    // The base path for this hook. When declared on
    // a site this should be the site project root.
    //
    // When declared on a plugin this should point
    // to th plugin base directory.
    base: PathBuf,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            args: None,
            stdout: Some(true),
            stderr: Some(true),
            after: None,
            profiles: None,
            files: None,
            watch: None,
            base: PathBuf::new(),
        }
    }
}

impl HookConfig {
    pub fn base(&self) -> &PathBuf {
        &self.base
    }
}
