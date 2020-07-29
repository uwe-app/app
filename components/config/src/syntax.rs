use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::profile::ProfileName;

static DEFAULT_THEME: &str = "base16-ocean.dark";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SyntaxConfig {
    pub enabled: bool,
    pub inline: Option<bool>,
    pub theme: Option<String>,
    pub languages: Option<Vec<String>>,
    pub map: Option<HashMap<String, String>>,
    pub profiles: Option<Vec<ProfileName>>,
}

impl Default for SyntaxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            inline: Some(true),
            languages: None,
            theme: Some(DEFAULT_THEME.to_string()),
            map: Some(HashMap::new()),
            profiles: None,
        }
    }
}

impl SyntaxConfig {

    pub fn is_enabled(&self, name: &ProfileName) -> bool {
        if self.enabled {
            if let Some(ref profiles) = self.profiles {
                return profiles.contains(name) 
            }
        }
        self.enabled 
    }

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
