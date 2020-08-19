use std::fmt;
use serde::{Deserialize, Serialize};

use utils::entity;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StyleSheetConfig {
    pub main: Vec<StyleFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StyleFile {
    Source(String),
    // NOTE: We may want to assign more fields when declaring
    // NOTE: stylesheets later, hence the enum!
    // NOTE: See: script.rs for an example.
}

impl StyleFile {
    pub fn get_source(&self) -> &str {
        match *self {
            Self::Source(ref s) => s,
        }
    }
}

impl fmt::Display for StyleFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Source(ref s) => {
                write!(f, "<link rel=\"stylesheet\" href=\"{}\">", entity::escape(s))?;
            }
        }
        Ok(())
    }
}
