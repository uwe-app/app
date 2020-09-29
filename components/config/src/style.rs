use serde::{Deserialize, Serialize};
use std::fmt;

use utils::entity;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StyleSheetConfig {
    pub main: Vec<StyleAsset>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum StyleAsset {
    Source(String),
    Inline { content: String },
    Tag(StyleTag),
}

impl StyleAsset {
    pub fn to_tag(&self) -> StyleTag {
        match *self {
            Self::Source(ref s) => StyleTag::new(s),
            Self::Tag(ref f) => f.clone(),
            Self::Inline { ref content } => StyleTag::new_content(content),
        }
    }

    pub fn get_source(&self) -> Option<&str> {
        match *self {
            Self::Source(ref s) => Some(s),
            Self::Tag(ref f) => {
                if let Some(ref href) = f.href {
                    Some(href)
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
                if let Some(ref mut href) = t.href {
                    t.href = Some(format!("{}/{}", base, href));
                }
            }
            Self::Inline { .. } => return false,
        }
        true
    }
}

impl fmt::Display for StyleAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let href: Option<&str> = match *self {
            Self::Source(ref s) => Some(s),

            Self::Tag(ref t) => {
                if let Some(ref href) = t.href {
                    Some(href)
                } else {
                    None
                }
            }
            _ => None,
        };

        let media: Option<&str> = match *self {
            Self::Tag(ref t) => {
                if let Some(ref media) = t.media {
                    Some(media)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(href) = href {
            if let Some(ref media) = media {
                write!(
                    f,
                    "<link rel=\"stylesheet\" href=\"{}\" media=\"{}\">",
                    entity::escape(href),
                    entity::escape(media)
                )?;
            } else {
                write!(
                    f,
                    "<link rel=\"stylesheet\" href=\"{}\">",
                    entity::escape(href)
                )?;
            }
        } else {
            match *self {
                Self::Tag(ref style) => {
                    if let Some(ref content) = style.content {
                        write!(f, "<style>{}</style>", content)?;
                    }
                }
                Self::Inline { ref content } => {
                    write!(f, "<style>{}</style>", content)?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct StyleTag {
    pub href: Option<String>,
    pub media: Option<String>,
    pub content: Option<String>,
}

impl StyleTag {
    pub fn new(s: &str) -> Self {
        Self {
            href: Some(s.to_string()),
            media: None,
            content: None,
        }
    }

    pub fn new_content(c: &str) -> Self {
        Self {
            href: None,
            media: None,
            content: Some(c.to_string()),
        }
    }
}
