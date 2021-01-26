use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::profile::{ProfileFilter, Profiles};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SyntaxConfig {
    pub theme: Option<String>,
    //pub languages: Option<Vec<String>>,
    pub map: Option<HashMap<String, String>>,
    profiles: ProfileFilter,
}

impl Default for SyntaxConfig {
    fn default() -> Self {
        Self {
            theme: None,
            //languages: None,
            map: Some(HashMap::new()),
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
}

impl Profiles for SyntaxConfig {
    fn profiles(&self) -> &ProfileFilter {
        &self.profiles
    }
}
