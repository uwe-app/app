use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::profile::ProfileName;

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

    pub(crate) fn prepare(&mut self) {
        for (k, v) in self.map.iter_mut() {
            if v.path.is_none() {
                v.path = Some(k.clone());
            }
            if v.stdout.is_none() {
                v.stdout = Some(true);
            }
            if v.stderr.is_none() {
                v.stderr = Some(true);
            }
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HookConfig {
    pub path: Option<String>,
    pub args: Option<Vec<String>>,
    pub source: Option<PathBuf>,
    pub stdout: Option<bool>,
    pub stderr: Option<bool>,
    // Marks the hook to run after a build
    pub after: Option<bool>,
    // Only run for these profiles
    pub profiles: Option<Vec<ProfileName>>,
}

impl HookConfig {
    pub fn get_source_path<P: AsRef<Path>>(&self, base: P) -> Option<PathBuf> {
        if let Some(ref src) = self.source {
            return Some(base.as_ref().to_path_buf().join(src));
        }
        None
    }
}
