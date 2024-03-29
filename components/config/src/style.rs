use std::fmt;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{href::UrlPath, tags::link::LinkTag};

use utils::entity;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum StyleAsset {
    Source(String),
    Inline { content: String },
    Tag(StyleTag),
}

impl StyleAsset {
    pub fn to_tag(self) -> StyleTag {
        match self {
            Self::Source(s) => StyleTag::new(s),
            Self::Tag(f) => f,
            Self::Inline { content } => StyleTag::new_content(content),
        }
    }

    pub fn source(&self) -> Option<&str> {
        match *self {
            Self::Source(ref s) => Some(s),
            Self::Tag(ref f) => Some(&f.href),
            Self::Inline { .. } => None,
        }
    }

    pub fn set_source_prefix(&mut self, base: &str) -> bool {
        match *self {
            Self::Source(ref mut s) => {
                *s = format!("{}/{}", base, s);
            }
            Self::Tag(ref mut t) => {
                t.href = format!("{}/{}", base, &t.href);
            }
            Self::Inline { .. } => return false,
        }
        true
    }

    pub fn dynamic(&self) -> bool {
        match *self {
            Self::Tag(ref tag) => tag.dynamic.is_some() && tag.dynamic.unwrap(),
            _ => false,
        }
    }
}

impl From<UrlPath> for StyleAsset {
    fn from(path: UrlPath) -> Self {
        StyleAsset::Source(path.to_string())
    }
}

impl fmt::Display for StyleAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let href: Option<&str> = match *self {
            Self::Source(ref s) => Some(s),

            Self::Tag(ref t) => Some(&t.href),
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
                    "<link rel=\"stylesheet\" href=\"{}\" media=\"{}\">\n",
                    entity::escape(href),
                    entity::escape(media)
                )?;
            } else {
                write!(
                    f,
                    "<link rel=\"stylesheet\" href=\"{}\">\n",
                    entity::escape(href)
                )?;
            }
        } else {
            match *self {
                Self::Tag(ref style) => {
                    if let Some(ref content) = style.content {
                        write!(f, "<style>{}</style>\n", content)?;
                    }
                }
                Self::Inline { ref content } => {
                    write!(f, "<style>{}</style>\n", content)?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct StyleTag {
    #[serde(alias = "src")]
    href: String,
    media: Option<String>,
    content: Option<String>,

    // Flag this style as dynamic so that we do not verify it must
    // exist in the source directory ar runtime.
    //
    // These styles are typically generated dynamically using an
    // external program via a hook.
    dynamic: Option<bool>,
}

impl StyleTag {
    pub fn new(href: String) -> Self {
        Self {
            href,
            media: None,
            content: None,
            dynamic: None,
        }
    }

    pub fn new_content(c: String) -> Self {
        Self {
            href: String::new(),
            media: None,
            content: Some(c),
            dynamic: None,
        }
    }

    pub fn to_link_tag(self) -> LinkTag {
        LinkTag::new_style_sheet(self.href, self.media)
    }

    pub fn source(&self) -> &str {
        &self.href
    }

    pub fn set_source<S: AsRef<str>>(&mut self, source: S) {
        self.href = source.as_ref().to_string();
    }
}

impl PartialEq for StyleTag {
    fn eq(&self, other: &Self) -> bool {
        self.href == other.href
    }
}

impl Eq for StyleTag {}
