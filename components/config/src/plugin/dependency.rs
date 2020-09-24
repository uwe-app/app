use std::collections::hash_map;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fmt;

use globset::{Glob, GlobMatcher};
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::Result;

use super::plugin::Plugin;

// TODO: spdx license for Plugin and ExternalLibrary

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyMap {
    #[serde(flatten)]
    pub items: HashMap<String, Dependency>,
}

impl DependencyMap {

    pub fn into_iter(self) -> hash_map::IntoIter<String, Dependency> {
        self.items.into_iter()
    }

    pub fn to_vec(&self) -> Vec<(&String, &Dependency)> {
        let out: Vec<(&String, &Dependency)> = Vec::new();
        self.items.iter().fold(out, |mut acc, (name, dep)| {

            if let Some(ref plugin) = dep.plugin {
                if let Some(ref dependencies) = plugin.dependencies {
                    let mut deps = dependencies.to_vec();
                    acc.append(&mut deps);
                }
            }

            acc.push((name, dep));
            acc
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DependencyTarget {
    /// Load plugin from a local folder.
    File{ path: PathBuf },
    /// Load plugin from a compressed archive.
    Archive{ archive: PathBuf },
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    /// Required version for the dependency.
    #[serde_as(as = "DisplayFromStr")]
    pub version: VersionReq,

    /// Path for a local file system plugin.
    #[serde(flatten)]
    pub target: DependencyTarget,

    /// Patterns that determine how styles, scripts and layouts 
    /// are applied to pages.
    pub apply: Option<Apply>,

    /// Resolved plugin for this dependency.
    #[serde(skip)]
    pub plugin: Option<Plugin>,

    /// Injected when resolving dependencies from the hash map key.
    #[serde(skip)]
    pub name: Option<String>,
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name.as_ref().unwrap(), self.version.to_string())
    }
}

impl Dependency {
    /// Cache glob patterns used to apply plugins to
    /// files.
    pub fn prepare(&mut self) -> Result<()> {
        if let Some(ref mut apply) = self.apply {
            apply.prepare()?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Apply {
    pub styles: Option<Vec<Glob>>,
    pub scripts: Option<Vec<Glob>>,
    pub layouts: Option<HashMap<String, Vec<Glob>>>,

    #[serde(skip)]
    pub styles_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub scripts_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub layouts_match: HashMap<String, Vec<GlobMatcher>>,
}

impl Apply {

    /// Prepare the global patterns by compiling them.
    ///
    /// Original GlobSet declarations are moved out of the Option(s).
    pub(crate) fn prepare(&mut self) -> Result<()> {
        self.styles_match = if let Some(styles) = self.styles.take() {
            styles.iter().map(|g| g.compile_matcher()).collect()
        } else { Vec::new() };

        self.scripts_match = if let Some(scripts) = self.scripts.take() {
            scripts.iter().map(|g| g.compile_matcher()).collect()
        } else { Vec::new() };

        self.layouts_match = if let Some(layouts) = self.layouts.take() {
            let mut tmp: HashMap<String, Vec<GlobMatcher>> = HashMap::new();
            for (k, v) in layouts {
                tmp.insert(k, v.iter().map(|g| g.compile_matcher()).collect());
            }
            tmp
        } else { HashMap::new() };
        Ok(())
    }
}
