use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{profile::ProfileName, utils::href::UrlPath, Error, Result};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HookMap {
    #[serde(flatten)]
    pub map: HashMap<String, HookConfig>,
}

impl HookMap {
    pub fn iter(
        &self,
    ) -> std::collections::hash_map::Iter<'_, String, HookConfig> {
        self.map.iter()
    }

    pub(crate) fn prepare(&mut self, base: &PathBuf) -> Result<()> {
        for (k, v) in self.map.iter_mut() {
            if v.path.is_empty() {
                return Err(Error::HookPathEmpty(k.to_string(), base.to_path_buf()))
            }
            v.base = base.to_path_buf();
        }

        Ok(())
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
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

/*
impl HookConfig {
    pub fn get_source_path<P: AsRef<Path>>(&self, base: P) -> Option<PathBuf> {
        if let Some(ref src) = self.source {
            return Some(base.as_ref().to_path_buf().join(src));
        }
        None
    }
}
*/
