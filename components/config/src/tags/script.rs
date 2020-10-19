use std::fmt;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use utils::entity;

use super::{CrossOrigin, ReferrerPolicy};

// SEE: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct ScriptTag {
    pub src: Option<String>,
    pub nomodule: Option<bool>,
    pub nonce: Option<String>,
    pub integrity: Option<String>,
    pub crossorigin: Option<CrossOrigin>,
    pub referrerpolicy: Option<ReferrerPolicy>,

    #[serde(rename = "type")]
    pub script_type: Option<String>,
    #[serde(rename = "async")]
    pub script_async: Option<bool>,

    pub content: Option<String>,
}

impl ScriptTag {
    pub fn new(s: &str) -> Self {
        Self {
            src: Some(s.to_string()),
            nomodule: None,
            nonce: None,
            integrity: None,
            crossorigin: None,
            referrerpolicy: None,
            script_type: None,
            script_async: None,
            content: None,
        }
    }

    pub fn new_content(c: &str) -> Self {
        Self {
            src: None,
            nomodule: None,
            nonce: None,
            integrity: None,
            crossorigin: None,
            referrerpolicy: None,
            script_type: None,
            script_async: None,
            content: Some(c.to_string()),
        }
    }
}

impl fmt::Display for ScriptTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref attr) = self.src {
            write!(f, "<script src=\"{}\"", entity::escape(attr))?;
        }
        if let Some(ref attr) = self.script_type {
            write!(f, " type=\"{}\"", entity::escape(attr))?;
        }
        if let Some(ref attr) = self.nonce {
            write!(f, " nonce=\"{}\"", entity::escape(attr))?;
        }
        if let Some(ref attr) = self.integrity {
            write!(f, " integrity=\"{}\"", entity::escape(attr))?;
        }
        if let Some(ref attr) = self.referrerpolicy {
            // NOTE: we know that we don't need to escape this attribute value
            write!(
                f,
                " referrerpolicy=\"{}\"",
                attr.as_str()
            )?;
        }

        if let Some(ref attr) = self.crossorigin {
            // NOTE: we know that we don't need to escape this attribute value
            write!(f, " {}", attr.as_str())?;
        }

        if let Some(_) = self.script_async { write!(f, " async")?; }
        if let Some(_) = self.nomodule { write!(f, " nomodule")?; }

        write!(f, ">")?;

        if let Some(ref content) = self.content {
            write!(f, "{}", content)?;
        }

        write!(f, "</script>\n")
    }
}


impl PartialEq for ScriptTag {
    fn eq(&self, other: &Self) -> bool {
        self.src == other.src
    }
}

impl Eq for ScriptTag {}

