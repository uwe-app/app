use std::fmt;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use utils::entity;

use crate::{href::UrlPath, tags::script::ScriptTag, tags::CrossOrigin};

// SEE: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JavaScriptConfig {
    pub main: Vec<ScriptAsset>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum ScriptAsset {
    Source(String),
    Inline { content: String },
    Tag(ScriptTag),
}

impl ScriptAsset {
    pub fn to_tag(&self) -> ScriptTag {
        match *self {
            Self::Source(ref s) => ScriptTag::new(s),
            Self::Tag(ref f) => f.clone(),
            Self::Inline { ref content } => ScriptTag::new_content(content),
        }
    }

    pub fn get_source(&self) -> Option<&str> {
        match *self {
            Self::Source(ref s) => Some(s),
            Self::Tag(ref f) => {
                if let Some(ref src) = f.src {
                    Some(src)
                } else {
                    None
                }
            }
            Self::Inline { .. } => None,
        }
    }

    pub fn set_source_prefix(&mut self, base: &str) -> bool {
        match *self {
            Self::Source(ref mut s) => {
                *s = format!("{}/{}", base, s);
            }
            Self::Tag(ref mut t) => {
                if let Some(ref mut src) = t.src {
                    t.src = Some(format!("{}/{}", base, src));
                }
            }
            Self::Inline { .. } => return false,
        }
        true
    }
}

impl From<UrlPath> for ScriptAsset {
    fn from(path: UrlPath) -> Self {
        ScriptAsset::Source(path.to_string())
    }
}

impl fmt::Display for ScriptAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Source(ref s) => {
                write!(f, "<script src=\"{}\"></script>", entity::escape(s))
            }
            Self::Inline { ref content } => {
                write!(f, "<script>{}</script>", content)
            }
            Self::Tag(ref script) => {
                script.fmt(f)
            }
        }
    }
}

