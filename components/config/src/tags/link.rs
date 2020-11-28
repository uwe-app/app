use std::fmt;

use serde::{Deserialize, Serialize};
use serde_with::{
    serde_as, skip_serializing_none, SpaceSeparator, StringWithSeparator,
};

use utils::entity;

use super::attr::{As, CrossOrigin, RelValue};

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Serialize, Deserialize, Clone, Hash)]
#[serde(default)]
pub struct LinkTag {
    // TODO: support enum for UrlPath | Url
    #[serde(alias = "src")]
    href: String,

    #[serde(rename = "as")]
    as_attr: Option<As>,
    crossorigin: Option<CrossOrigin>,
    disabled: Option<bool>,

    #[serde(rename = "hreflang")]
    href_lang: Option<String>,

    #[serde(rename = "imagesizes")]
    image_sizes: Option<String>,

    #[serde(rename = "imagesrcset")]
    image_src_set: Option<String>,

    media: Option<String>,
    prefetch: Option<bool>,

    sizes: Option<String>,
    title: Option<String>,

    #[serde(rename = "type")]
    link_type: Option<String>,

    #[serde_as(as = "Option<StringWithSeparator::<SpaceSeparator, RelValue>>")]
    rel: Option<Vec<RelValue>>,

    // Events
    onload: Option<String>,
}

impl LinkTag {
    pub fn new_style_sheet(href: String, media: Option<String>) -> Self {
        Self {
            href,
            media,
            rel: Some(vec![RelValue::StyleSheet]),
            ..Default::default()
        }
    }

    pub fn new_icon(href: String) -> Self {
        Self {
            href,
            rel: Some(vec![RelValue::Icon]),
            ..Default::default()
        }
    }

    pub fn new_canonical(href: String) -> Self {
        Self {
            href,
            rel: Some(vec![RelValue::Canonical]),
            ..Default::default()
        }
    }

    pub fn new_bookmark(href: String) -> Self {
        Self {
            href,
            rel: Some(vec![RelValue::Bookmark]),
            ..Default::default()
        }
    }

    pub fn source(&self) -> &str {
        &self.href
    }

    pub fn set_source(&mut self, val: String) {
        self.href = val;
    }
}

impl fmt::Display for LinkTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<link")?;

        if let Some(ref rel) = self.rel {
            let values = rel.iter().map(|r| r.as_str()).collect::<Vec<_>>();
            let attr = values.join(" ");
            write!(f, " rel=\"{}\"", entity::escape(&attr))?;
        }

        write!(f, " href=\"{}\"", entity::escape(&self.href))?;

        if let Some(ref attr) = self.as_attr {
            write!(f, " as=\"{}\"", entity::escape(attr.as_str()))?;
        }

        if let Some(ref attr) = self.crossorigin {
            write!(f, " crossorigin=\"{}\"", entity::escape(attr.as_str()))?;
        }

        if let Some(ref attr) = self.href_lang {
            write!(f, " hreflang=\"{}\"", entity::escape(attr))?;
        }

        if let Some(ref attr) = self.image_sizes {
            write!(f, " imagesizes=\"{}\"", entity::escape(attr))?;
        }

        if let Some(ref attr) = self.image_src_set {
            write!(f, " imagesrcset=\"{}\"", entity::escape(attr))?;
        }

        if let Some(ref attr) = self.media {
            write!(f, " media=\"{}\"", entity::escape(attr))?;
        }

        if let Some(ref attr) = self.sizes {
            write!(f, " sizes=\"{}\"", entity::escape(attr))?;
        }

        if let Some(ref attr) = self.title {
            write!(f, " title=\"{}\"", entity::escape(attr))?;
        }

        if let Some(ref attr) = self.link_type {
            write!(f, " type=\"{}\"", entity::escape(attr))?;
        }

        if let Some(ref attr) = self.onload {
            write!(f, " onload=\"{}\"", entity::escape(attr))?;
        }

        if let Some(_) = self.disabled {
            write!(f, " disabled")?;
        }
        if let Some(_) = self.prefetch {
            write!(f, " prefetch")?;
        }

        write!(f, ">")
    }
}

impl PartialEq for LinkTag {
    fn eq(&self, other: &Self) -> bool {
        self.href == other.href
    }
}

impl Eq for LinkTag {}
