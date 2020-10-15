use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::profile::ProfileName;

static DEFAULT_THEME: &str = "base16-ocean.light";

use crate::profile::{Profiles, ProfileFilter};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SyntaxConfig {
    pub inline: Option<bool>,
    pub theme: Option<String>,
    //pub languages: Option<Vec<String>>,
    pub map: Option<HashMap<String, String>>,
    profiles: ProfileFilter,
}

impl Default for SyntaxConfig {
    fn default() -> Self {
        Self {
            inline: Some(true),
            //languages: None,
            theme: Some(DEFAULT_THEME.to_string()),
            map: Some(HashMap::new()),
            profiles: Default::default(),
        }
    }
}

impl SyntaxConfig {

    pub fn is_inline(&self) -> bool {
        self.inline.is_some() && self.inline.unwrap()
    }

    pub fn theme(&self) -> &str {
        if let Some(ref theme) = self.theme {
            theme
        } else {
            DEFAULT_THEME
        }
    }
}

impl Profiles for SyntaxConfig {

    fn has_profile(&self, name: &ProfileName) -> bool {
        match self.profiles {
            ProfileFilter::Flag(enabled) => enabled,
            ProfileFilter::Name(ref target) => target == name,
            ProfileFilter::List(ref target) => target.contains(name),
        } 
    }
}
