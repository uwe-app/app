use std::path::PathBuf;
use std::collections::HashMap;

//use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};

static DEFAULT_FROM_PATH: &str = "/";
static DEFAULT_NAME: &str = "feed";
static JSON: &str = "json";
static XML: &str = "xml";

static RSS_SUFFIX: &str = "-rss";
static ATOM_SUFFIX: &str = "-atom";

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum FeedType {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "rss")]
    Rss,
    #[serde(rename = "atom")]
    Atom,
}

impl FeedType {
    pub fn get_suffix(&self) -> &str {
        match *self {
            Self::Rss => RSS_SUFFIX,
            Self::Atom => ATOM_SUFFIX,
            _ => ""
        } 
    }

    pub fn get_extension(&self) -> &str {
        match *self {
            Self::Json => JSON,
            _ => XML
        } 
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FeedConfig {
    // List of custom templates to use for each feed.
    //
    // When specified they override the default templates 
    // for each feed type.
    pub templates: HashMap<FeedType, PathBuf>,

    #[serde(flatten)]
    pub channels: HashMap<String, ChannelConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ChannelConfig {
    // The path to include feed pages from, eg: `/posts`.
    //
    // Feed files will be placed in this directory.
    pub from: Option<String>,

    // The name of the feed indicates the name for each
    // output file.
    pub name: Option<String>,

    // List of file types to generate for this feed
    pub types: Vec<FeedType>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            from: Some(DEFAULT_FROM_PATH.to_string()),
            name: Some(DEFAULT_NAME.to_string()), 
            types: vec![FeedType::Json, FeedType::Rss],
        }
    }
}
