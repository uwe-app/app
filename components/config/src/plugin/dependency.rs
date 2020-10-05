use std::collections::hash_map;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;

use globset::{Glob, GlobMatcher};
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::{Error, Result};

use super::features::{FeatureFlags, FeatureMap};

static FEATURE_STACK_SIZE: usize = 16;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum AccessGrant {
    #[serde(rename = "hooks")]
    Hooks,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyMap {
    #[serde(flatten, serialize_with = "toml::ser::tables_last")]
    items: HashMap<String, Dependency>,
}

impl DependencyMap {

    pub fn entry(&mut self, name: String) -> hash_map::Entry<'_, String, Dependency> {
        self.items.entry(name)
    }

    pub fn into_iter(self) -> hash_map::IntoIter<String, Dependency> {
        self.items.into_iter()
    }

    pub fn iter(&self) -> hash_map::Iter<'_, String, Dependency> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> hash_map::IterMut<'_, String, Dependency> {
        self.items.iter_mut()
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

    pub fn append(&mut self, other: DependencyMap) {
        other.into_iter().for_each(|(k, v)| {
            self.items.insert(k, v);
        });
    }

    /// Recursive feature resolver.
    fn resolver(
        &self,
        src: &Dependency,
        map: &FeatureMap,
        features: &Vec<String>,
        out: &mut DependencyMap,
        stack: &mut Vec<String>,
    ) -> Result<()> {
        features.iter().try_for_each(|n| {
            if stack.len() > FEATURE_STACK_SIZE {
                return Err(Error::FeatureStackTooLarge(FEATURE_STACK_SIZE));
            } else if stack.contains(n) {
                return Err(Error::CyclicFeature(n.to_string()));
            }

            if let Some(dep) = self.get(n) {
                out.items.insert(n.clone(), dep.clone());
            } else if let Some(item) = map.get(n) {
                stack.push(n.clone());
                self.resolver(src, map, item, out, stack)?;
                stack.pop();
            } else {
                return Err(Error::NoFeature(src.to_string(), n.to_string()));
            }
            Ok(())
        })?;

        Ok(())
    }

    /// Resolve feature flags.
    fn resolve(
        &self,
        src: &Dependency,
        map: &FeatureMap,
        features: &Vec<String>,
    ) -> Result<DependencyMap> {
        let mut out: DependencyMap = Default::default();
        self.resolver(src, map, features, &mut out, &mut Default::default())?;
        Ok(out)
    }

    /// Filter this dependency map using the feature flags from a
    /// source dependency.
    pub fn filter(
        &self,
        src: &Dependency,
        map: &FeatureMap,
    ) -> Result<DependencyMap> {

        let flags = &src.features;

        let mut out: DependencyMap = Default::default();

        // Collect non-optional dependencies
        self.iter()
            .filter(|(_, d)| !d.is_optional())
            .for_each(|(k, d)| {
                out.items.insert(k.to_string(), d.clone());
            });

        // Determine if we need default features
        let default_features = if let Some(ref flags) = flags {
            flags.use_default_features()
        } else {
            true
        };

        // Collect default features if available
        let defaults = if !map.is_empty() {
            map.default()
        } else {
            None
        };

        // Assign default features if required and available
        if default_features {
            if let Some(default) = defaults {
                let deps = self.resolve(src, map, default)?;
                out.append(deps);
            }
        }

        // Resolve requested features
        if let Some(ref specs) = flags {
            if let Some(ref include_flags) = specs.flags {
                let deps = self.resolve(src, map, include_flags)?;
                out.append(deps);
            }
        }

        Ok(out)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DependencyTarget {
    /// Load plugin from a local folder.
    File { path: PathBuf },
    /// Load plugin from a compressed archive.
    Archive { archive: PathBuf },
    /// Load plugin from a git repository.
    Repo { git: String },
    /// Load plugin from a local scope.
    Local { scope: String },
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

    /// Grant permissions to the plugin.
    enable: Option<HashSet<AccessGrant>>,
}

impl Dependency {
    pub fn new_scope(scope: String, version: VersionReq ) -> Self {
        Self {
            version,
            target: Some(DependencyTarget::Local { scope }),
            optional: Some(true),
            name: None,
            features: None,
            apply: None,
            enable: None,
        }
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.name {
            write!(f, "{}@{}", name, self.version.to_string())
        } else {
            write!(f, "{}", self.version.to_string())
        }
    }
}

impl Dependency {
    /// Determine if this dependency has the given access granted.
    pub fn grants(&self, access: AccessGrant) -> bool {
        if let Some(ref grants) = self.enable {
            return grants.contains(&access);
        }
        false
    }

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
