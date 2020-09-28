use std::collections::HashMap;

use serde::{Deserialize, Serialize};

type FeatureName = String;
type DependencyName = String;

/// Flags used by a dependency to select optional dependencies.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct FeatureFlags {
    /// Enable or disable the default features for a dependency.
    pub default_features: Option<bool>,

    /// Explicit list of feature flags so that dependencies can be filtered 
    /// by optionality.
    pub flags: Option<Vec<FeatureName>>,
}

/// Map of features to dependencies used by plugin definitions 
/// to indicate which dependencies should be resolved for a given 
/// set of feature flags.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureMap {
    #[serde(flatten)]
    pub map: Option<HashMap<FeatureName, Vec<DependencyName>>>,
    pub default: Vec<DependencyName>,
}
