use std::fmt;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, StringWithSeparator, SpaceSeparator};

use super::attr::{RelValue, CrossOrigin, As};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkTag {
    #[serde(rename = "as")]
    as_attr: Option<As>,
    crossorigin: Option<CrossOrigin>,
    disabled: Option<bool>,
    href: String,

    #[serde(rename = "hreflang")]
    href_lang: Option<String>,

    #[serde(rename = "imagesizes")]
    image_sizes: Option<String>,

    #[serde(rename = "imagesrcset")]
    image_src_set: Option<String>,

    #[serde(rename = "imagesrcset")]
    media: Option<String>,

    prefetch: Option<bool>,

    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, RelValue>")]
    rel: Vec<RelValue>,

    sizes: Option<String>,
    title: Option<String>,

    #[serde(rename = "type")]
    link_type: Option<String>,

    // Events
    onload: Option<String>,
}

impl fmt::Display for LinkTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //write!(f, "{}", self.to_string())
        Ok(())
    }
}

