use serde::{Deserialize, Serialize};
use std::fmt;

use utils::entity;

// SEE: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JavaScriptConfig {
    pub main: Vec<ScriptAsset>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ScriptAsset {
    Source(String),
    Tag(ScriptTag),
    Inline { content: String },
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CrossOrigin {
    #[serde(rename = "anonymous")]
    Anonymous,
    #[serde(rename = "use-credentials")]
    UseCredentials,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
