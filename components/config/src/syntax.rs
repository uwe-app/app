use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::profile::{ProfileFilter, Profiles};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SyntaxConfig {
    theme: Option<String>,
    profiles: ProfileFilter,
    map: HashMap<String, String>,
}

impl Default for SyntaxConfig {
    fn default() -> Self {
        Self {
            theme: None,
            map: HashMap::new(),
            profiles: Default::default(),
        }
    }
}

impl SyntaxConfig {
    pub fn is_inline(&self) -> bool {
        self.theme.is_some()
    }

    pub fn theme(&self) -> &Option<String> {
        &self.theme
    }

    pub fn map(&self) -> &HashMap<String, String> {
        &self.map
    }
}

impl Profiles for SyntaxConfig {
    fn profiles(&self) -> &ProfileFilter {
        &self.profiles
    }
}
