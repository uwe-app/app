use serde::{Deserialize, Serialize};

static DEFAULT_THEME: &str = "base16-ocean.dark";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SyntaxConfig {
    pub inline: Option<bool>,
    pub theme: Option<String>,
    pub languages: Option<Vec<String>>,
}

impl Default for SyntaxConfig {
    fn default() -> Self {
        Self {
            inline: Some(false),
            languages: None,
            theme: Some(DEFAULT_THEME.to_string()),
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
