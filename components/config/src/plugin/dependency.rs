use std::collections::hash_map;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use globset::{Glob, GlobMatcher};
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::Result;

use super::features::{FeatureFlags, FeatureMap};

// TODO: spdx license for Plugin and ExternalLibrary

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyMap {
    #[serde(flatten)]
    items: HashMap<String, Dependency>,
}

impl DependencyMap {
    pub fn into_iter(self) -> hash_map::IntoIter<String, Dependency> {
        self.items.into_iter()
    }

    pub fn iter(&self) -> hash_map::Iter<'_, String, Dependency> {
        self.items.iter()
    }

    pub fn keys(&self) -> hash_map::Keys<String, Dependency> {
        self.items.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get<S: AsRef<str>>(&self, s: S) -> Option<&Dependency> {
        self.items.get(s.as_ref()) 
    }

    pub fn contains_key<S: AsRef<str>>(&self, s: S) -> bool {
        self.items.contains_key(s.as_ref()) 
    }

    /// Filter this dependency map using the feature flags from a 
    /// source dependency.
    pub fn filter(self, flags: &Option<FeatureFlags>, map: &Option<FeatureMap>) -> DependencyMap {
        if let Some(ref flags) = flags {
            let default_features = flags.default_features.is_none()
                || (flags.default_features.is_some() && flags.default_features.unwrap());

            println!("Flags {:#?}", flags);
            println!("Map {:#?}", map);
        }
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DependencyTarget {
    /// Load plugin from a local folder.
    File { path: PathBuf },
    /// Load plugin from a compressed archive.
    Archive { archive: PathBuf },
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Dependency {
    /// Injected when resolving dependencies from the hash map key or 
    /// converting from lock file entries or references.
    #[serde(skip)]
    pub name: Option<String>,

    /// Required version for the dependency.
    #[serde_as(as = "DisplayFromStr")]
    pub version: VersionReq,

    /// Indicates this dependency is optional and may 
    /// be activated via a feature flag.
    pub optional: Option<bool>,

    #[serde(flatten)]
    pub features: Option<FeatureFlags>,

    /// Optional target such as a folder, archive or git repository.
    #[serde(flatten)]
    pub target: Option<DependencyTarget>,

    /// Patterns that determine how styles, scripts and layouts
    /// are applied to pages.
    pub apply: Option<Apply>,

}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.name {
            write!(
                f,
                "{}@{}",
                name,
                self.version.to_string()
            )
        } else {
            write!(
                f,
                "{}",
                self.version.to_string()
            )
        }
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

    pub fn is_optional(&self) -> bool {
        self.optional.is_some() && self.optional.unwrap()
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
        } else {
            Vec::new()
        };

        self.scripts_match = if let Some(scripts) = self.scripts.take() {
            scripts.iter().map(|g| g.compile_matcher()).collect()
        } else {
            Vec::new()
        };

        self.layouts_match = if let Some(layouts) = self.layouts.take() {
            let mut tmp: HashMap<String, Vec<GlobMatcher>> = HashMap::new();
            for (k, v) in layouts {
                tmp.insert(k, v.iter().map(|g| g.compile_matcher()).collect());
            }
            tmp
        } else {
            HashMap::new()
        };
        Ok(())
    }
}
