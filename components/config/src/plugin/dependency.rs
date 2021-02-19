use std::collections::hash_map;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::path::PathBuf;

use globset::{Glob, GlobMatcher};
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use url::Url;

use crate::{utils::href::UrlPath, Error, Result};

use super::features::{FeatureFlags, FeatureMap};
use super::plugin_spec::{ExactPluginSpec, PluginSpec};

const FEATURE_STACK_SIZE: usize = 16;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyDefinitionMap {
    #[serde(flatten, serialize_with = "toml::ser::tables_last")]
    items: HashMap<String, DependencyDefinition>,
}

impl TryInto<DependencyMap> for DependencyDefinitionMap {
    type Error = Error;

    fn try_into(self) -> std::result::Result<DependencyMap, Self::Error> {
        let mut map: DependencyMap = Default::default();
        for (name, def) in self.items.into_iter() {
            let dep = match def {
                DependencyDefinition::VersionRange(range) => {
                    let version: VersionReq = range.parse()?;
                    let dep = Dependency::new(version);
                    dep
                }
                DependencyDefinition::Dependency(dep) => dep,
            };
            map.items.insert(name, dep);
        }
        Ok(map)
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DependencyDefinition {
    VersionRange(String),
    Dependency(Dependency),
}

impl Default for DependencyDefinition {
    fn default() -> Self {
        Self::VersionRange(String::new())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, Eq, PartialEq)]
pub struct DependencyMap {
    #[serde(flatten, serialize_with = "toml::ser::tables_last")]
    items: HashMap<String, Dependency>,
}

impl DependencyMap {
    pub fn entry(
        &mut self,
        name: String,
    ) -> hash_map::Entry<'_, String, Dependency> {
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

    pub fn len(&self) -> usize {
        self.items.len()
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
        let defaults = if !map.is_empty() { map.default() } else { None };

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

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum DependencyTarget {
    /// Load plugin from a local folder.
    File { path: PathBuf },
    /// Load plugin from a compressed archive.
    Archive { archive: PathBuf },
    /// Load plugin from a git repository.
    Repo {
        #[serde_as(as = "DisplayFromStr")]
        git: Url,
        prefix: Option<UrlPath>,
    },
    /// Load plugin from a local scope.
    Local { scope: String },
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Dependency {
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

impl From<DependencyTarget> for Dependency {
    fn from(target: DependencyTarget) -> Self {
        Self {
            version: VersionReq::any(),
            target: Some(target),
            optional: None,
            features: None,
            apply: None,
        }
    }
}

impl Dependency {
    pub fn new(version: VersionReq) -> Self {
        Self {
            version,
            target: None,
            optional: None,
            features: None,
            apply: None,
        }
    }

    pub fn new_target(target: DependencyTarget) -> Self {
        Self {
            version: VersionReq::any(),
            target: Some(target),
            optional: None,
            features: None,
            apply: None,
        }
    }

    pub fn new_scope(scope: String, version: VersionReq) -> Self {
        Self {
            version,
            target: Some(DependencyTarget::Local { scope }),
            optional: Some(true),
            features: None,
            apply: None,
        }
    }

    pub fn target(&self) -> &Option<DependencyTarget> {
        &self.target
    }

    pub fn range(&self) -> &VersionReq {
        &self.version
    }

    pub fn set_range(&mut self, range: VersionReq) {
        self.version = range;
    }

    pub fn apply(&self) -> &Option<Apply> {
        &self.apply
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version.to_string())
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

impl From<PluginSpec> for Dependency {
    fn from(spec: PluginSpec) -> Self {
        Self {
            version: spec.range,
            apply: None,
            features: None,
            optional: None,
            target: None,
        }
    }
}

impl From<ExactPluginSpec> for Dependency {
    fn from(spec: ExactPluginSpec) -> Self {
        let version = if let Some(ref version) = spec.version {
            VersionReq::exact(version)
        } else {
            VersionReq::any()
        };

        Self {
            version,
            apply: None,
            features: None,
            optional: None,
            target: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum ApplyMatch {
    Pattern(Glob),
    Filter { to: Glob, filter: Glob },
}

impl Default for ApplyMatch {
    fn default() -> Self {
        Self::Pattern(Glob::new("**").unwrap())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Apply {
    pub styles: Option<Vec<ApplyMatch>>,
    pub scripts: Option<Vec<ApplyMatch>>,
    pub layouts: Option<HashMap<String, Vec<Glob>>>,

    #[serde(skip)]
    pub styles_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub scripts_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub layouts_match: HashMap<String, Vec<GlobMatcher>>,

    #[serde(skip)]
    pub styles_filter: Option<Vec<GlobMatcher>>,
    #[serde(skip)]
    pub scripts_filter: Option<Vec<GlobMatcher>>,
}

impl PartialEq for Apply {
    fn eq(&self, other: &Self) -> bool {
        self.styles == other.styles
            && self.scripts == other.scripts
            && self.layouts == other.layouts
    }
}

impl Eq for Apply {}

impl Apply {
    /// Prepare the global patterns by compiling them.
    pub(crate) fn prepare(&mut self) -> Result<()> {
        let (styles_match, styles_filter) =
            if let Some(ref styles) = self.styles {
                let mut styles_match = Vec::new();
                let mut styles_filter = Vec::new();
                for v in styles {
                    match v {
                        ApplyMatch::Pattern(ptn) => {
                            styles_match.push(ptn.compile_matcher());
                        }
                        ApplyMatch::Filter { ref to, ref filter } => {
                            styles_match.push(to.compile_matcher());
                            styles_filter.push(filter.compile_matcher());
                        }
                    }
                }

                if styles_filter.is_empty() {
                    (styles_match, None)
                } else {
                    (styles_match, Some(styles_filter))
                }
            } else {
                (Vec::new(), None)
            };

        self.styles_match = styles_match;
        self.styles_filter = styles_filter;

        let (scripts_match, scripts_filter) =
            if let Some(ref scripts) = self.scripts {
                let mut scripts_match = Vec::new();
                let mut scripts_filter = Vec::new();
                for v in scripts {
                    match v {
                        ApplyMatch::Pattern(ptn) => {
                            scripts_match.push(ptn.compile_matcher());
                        }
                        ApplyMatch::Filter { ref to, ref filter } => {
                            scripts_match.push(to.compile_matcher());
                            scripts_filter.push(filter.compile_matcher());
                        }
                    }
                }

                if scripts_filter.is_empty() {
                    (scripts_match, None)
                } else {
                    (scripts_match, Some(scripts_filter))
                }
            } else {
                (Vec::new(), None)
            };

        self.scripts_match = scripts_match;
        self.scripts_filter = scripts_filter;

        self.layouts_match = if let Some(ref layouts) = self.layouts {
            let mut tmp: HashMap<String, Vec<GlobMatcher>> = HashMap::new();
            for (k, v) in layouts {
                tmp.insert(
                    k.clone(),
                    v.iter().map(|g| g.compile_matcher()).collect(),
                );
            }
            tmp
        } else {
            HashMap::new()
        };

        Ok(())
    }

    pub fn has_scripts(&self) -> bool {
        !self.scripts_match.is_empty()
    }

    pub fn has_styles(&self) -> bool {
        !self.styles_match.is_empty()
    }

    /// Determine if any filters should be applied
    /// when assigning scripts.
    pub fn has_script_filters(&self) -> bool {
        if let Some(ref filters) = self.scripts_filter {
            !filters.is_empty()
        } else {
            false
        }
    }

    /// Determine if a script apply pattern matches all scripts
    /// defined by the plugin.
    pub fn has_script_wildcard(&self) -> bool {
        if let Some(ref scripts) = self.scripts {
            return scripts
                .iter()
                .find(|s| {
                    if let ApplyMatch::Pattern(_) = s {
                        true
                    } else {
                        false
                    }
                })
                .is_some();
        }
        false
    }

    /// Determine if any filters should be applied
    /// when assigning styles.
    pub fn has_style_filters(&self) -> bool {
        if let Some(ref filters) = self.styles_filter {
            !filters.is_empty()
        } else {
            false
        }
    }

    /// Determine if a style apply pattern matches all styles
    /// defined by the plugin.
    pub fn has_style_wildcard(&self) -> bool {
        if let Some(ref styles) = self.styles {
            return styles
                .iter()
                .find(|s| {
                    if let ApplyMatch::Pattern(_) = s {
                        true
                    } else {
                        false
                    }
                })
                .is_some();
        }
        false
    }
}
