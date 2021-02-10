use std::collections::{hash_set, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{profile::ProfileName, Error, Result};

#[derive(Debug, Clone)]
pub struct HookMap {
    exec: HashSet<HookConfig>,
}

impl From<HashSet<HookConfig>> for HookMap {
    fn from(exec: HashSet<HookConfig>) -> Self {
        Self { exec }
    }
}

impl HookMap {
    pub fn exec(&self) -> &HashSet<HookConfig> {
        &self.exec
    }

    pub fn iter(&self) -> hash_set::Iter<'_, HookConfig> {
        self.exec.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.exec.is_empty()
    }

    pub fn prepare(&mut self, source: &PathBuf, base: &PathBuf) -> Result<()> {
        let mut out: HashSet<HookConfig> = HashSet::new();
        for mut v in self.exec.drain() {
            if v.command.is_empty() {
                return Err(Error::HookPathEmpty(base.to_path_buf()));
            }

            v.base = base.canonicalize()?;
            v.source = source.canonicalize()?;

            // Check that hook files exist
            if let Some(ref files) = v.files {
                for f in files.iter() {
                    v.files_match.push(f.compile_matcher());
                }
            }

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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct HookConfig {
    // Command path.
    #[serde(alias = "path")]
    pub command: String,

    // Command arguments.
    pub args: Option<Vec<String>>,

    // Marks the hook to run after a build
    pub after: Option<bool>,

    // Only run for these profiles
    pub profiles: Option<Vec<ProfileName>>,

    // List of files expected by this hook
    pub files: Option<Vec<Glob>>,

    // Whether to trigger the hook when the
    // expected files changes.
    pub watch: Option<bool>,

    pub stdout: Option<bool>,
    pub stderr: Option<bool>,

    // Compiled files match patterns.
    #[serde(skip)]
    files_match: Vec<GlobMatcher>,

    // The base path for this hook. When declared on
    // a site this should be the site project root.
    //
    // When declared on a plugin this should point
    // to th plugin base directory.
    #[serde(skip)]
    base: PathBuf,

    // The source path is use to create relative paths
    // for glob pattern matching during live reload.
    #[serde(skip)]
    source: PathBuf,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: None,
            stdout: Some(true),
            stderr: Some(true),
            after: Some(false),
            profiles: None,
            files: None,
            watch: None,
            files_match: Vec::new(),
            base: PathBuf::new(),
            source: PathBuf::new(),
        }
    }
}

impl HookConfig {
    pub fn base(&self) -> &PathBuf {
        &self.base
    }

    pub fn has_matchers(&self) -> bool {
        !self.files_match.is_empty()
    }

    // Filter a list of changed files to the files that match
    // the patterns assigned to this hook.
    pub fn filter(&self, paths: &Vec<PathBuf>) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        for file in paths {
            if let Ok(file) = file.canonicalize() {
                if let Ok(relative) = file.strip_prefix(&self.source) {
                    for m in self.files_match.iter() {
                        if m.is_match(&relative) {
                            out.push(file.to_path_buf());
                        }
                    }
                }
            }
        }
        out
    }
}

// NOTE We have to implement these manually as GlobMatcher does not support
// NOTE: Hash, PartialEq etc and we want to prepare the matchers ahead of time.

impl PartialEq for HookConfig {
    fn eq(&self, other: &Self) -> bool {
        self.command == other.command && self.args == other.args
    }
}

impl Eq for HookConfig {}

impl Hash for HookConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.command.hash(state);
        self.args.hash(state);
    }
}
