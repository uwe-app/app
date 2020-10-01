use std::fmt;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use utils::entity;

use crate::href::UrlPath;

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
                write!(f, "<script src=\"{}\">", entity::escape(s))?;
            }
            Self::Inline { ref content } => {
                write!(f, "<script>{}</script>", content)?;
            }
            Self::Tag(ref script) => {
                if let Some(ref src) = script.src {
                    write!(f, "<script src=\"{}\"", entity::escape(src))?;
                } else {
                    write!(f, "<script")?;
                    if let Some(ref content) = script.content {
                        write!(f, ">{}</script>", content)?;
                        return Ok(());
                    }
                }

                if let Some(ref script_type) = script.script_type {
                    write!(f, " type=\"{}\"", entity::escape(script_type))?;
                }
                if let Some(ref nonce) = script.nonce {
                    write!(f, " nonce=\"{}\"", entity::escape(nonce))?;
                }
                if let Some(ref integrity) = script.integrity {
                    write!(f, " integrity=\"{}\"", entity::escape(integrity))?;
                }
                if let Some(ref referrer_policy) = script.referrerpolicy {
                    // NOTE: we know that we don't need to escape this attribute value
                    write!(
                        f,
                        " referrerpolicy=\"{}\"",
                        referrer_policy.to_string()
                    )?;
                }
                if let Some(ref _script_async) = script.script_async {
                    write!(f, " async")?;
                }
                if let Some(ref _nomodule) = script.nomodule {
                    write!(f, " nomodule")?;
                }
                if let Some(ref cross_origin) = script.crossorigin {
                    match *cross_origin {
                        CrossOrigin::Anonymous => {
                            write!(f, " crossorigin")?;
                        }
                        CrossOrigin::UseCredentials => {
                            write!(f, " crossorigin=\"use-credentials\"")?;
                        }
                    }
                }
                write!(f, ">")?
            }
        }
        write!(f, "</script>")
    }
}

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

impl PartialEq for ScriptTag {
    fn eq(&self, other: &Self) -> bool {
        self.src == other.src
    }
}

impl Eq for ScriptTag {}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum CrossOrigin {
    #[serde(rename = "anonymous")]
    Anonymous,
    #[serde(rename = "use-credentials")]
    UseCredentials,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum ReferrerPolicy {
    #[serde(rename = "no-referrer")]
    NoReferrer,
    #[serde(rename = "no-referrer-when-downgrade")]
    NoReferrerWhenDowngrade,
    #[serde(rename = "origin")]
    Origin,
    #[serde(rename = "origin-when-cross-origin")]
    OriginWhenCrossOrigin,
    #[serde(rename = "same-origin")]
    SameOrigin,
    #[serde(rename = "strict-origin")]
    StrictOrigin,
    #[serde(rename = "strict-origin-when-cross-origin")]
    StrictOriginWhenCrossOrigin,
    // NOTE: there is also unsafe-url but we prefer to avoid unsafe
}

impl fmt::Display for ReferrerPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NoReferrer => write!(f, "no-referrer"),
            Self::NoReferrerWhenDowngrade => {
                write!(f, "no-referrer-when-downgrade")
            }
            Self::Origin => write!(f, "origin"),
            Self::OriginWhenCrossOrigin => {
                write!(f, "origin-when-cross-origin")
            }
            Self::SameOrigin => write!(f, "same-origin"),
            Self::StrictOrigin => write!(f, "strict-origin"),
            Self::StrictOriginWhenCrossOrigin => {
                write!(f, "strict-origin-when-cross-origin")
            }
        }
    }
}

impl Default for ReferrerPolicy {
    fn default() -> Self {
        Self::NoReferrerWhenDowngrade
    }
}
