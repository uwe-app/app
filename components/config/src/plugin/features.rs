use std::collections::{HashMap, HashSet};
use std::collections::hash_map;
use std::iter::FromIterator;

use serde::{Deserialize, Serialize};

type FeatureName = String;
type DependencyName = String;

static DEFAULT: &str = "default";

/// Flags used by a dependency to select optional dependencies.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct FeatureFlags {
    /// Enable or disable the default features for a dependency.
    pub default_features: Option<bool>,

    /// Explicit list of feature flags so that dependencies can be filtered 
    /// by optionality.
    #[serde(rename = "features")]
    pub flags: Option<Vec<FeatureName>>,
}

/// Map of features to dependencies used by plugin definitions 
/// to indicate which dependencies should be resolved for a given 
/// set of feature flags.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FeatureMap {
    #[serde(flatten)]
    map: HashMap<FeatureName, Vec<DependencyName>>,
}

impl FeatureMap {

    pub fn get(&self, name: &FeatureName) -> Option<&Vec<DependencyName>> {
        self.map.get(name) 
    }

    pub fn default(&self) -> Option<&Vec<DependencyName>> {
        self.map.get(DEFAULT) 
    }

    pub fn iter(&self) -> hash_map::Iter<'_, FeatureName, Vec<DependencyName>> {
        self.map.iter() 
    }

    pub fn contains_key<S: AsRef<str>>(&self, s: S) -> bool {
        self.map.contains_key(s.as_ref())
    }

    /// Resolve feature names to a set of expected dependency names.
    pub fn names<'a>(&'a self) -> HashSet<&'a String> {
        let flat: Vec<&String> = self.map
            .iter()
            .flat_map(|(_, v)| v)
            .flat_map(|n| {
                if self.map.contains_key(n) {
                    return self.map
                        .get(n)
                        .unwrap()
                        .iter()
                        .collect::<Vec<&String>>()
                        .into_iter();
                }
                vec![n].into_iter()
            })
            .collect();
        HashSet::from_iter(flat.into_iter())
    }
}
