use std::path::PathBuf;
use std::collections::HashMap;

use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};

static JSON: &str = "json";
static XML: &str = "xml";

static FEED_NAME: &str = "feed";
static ATOM_NAME: &str = "atom";

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
    pub fn get_name(&self) -> &str {
        match *self {
            Self::Rss | Self::Json => FEED_NAME,
            Self::Atom => ATOM_NAME,
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

impl FeedConfig {
    // Prepare the configuration by compiling the glob matchers.
    pub fn prepare(&mut self) {
        for (_k, v) in self.channels.iter_mut() {
            v.include_match = v.includes.iter().map(|g| g.compile_matcher()).collect();
            v.exclude_match = v.excludes.iter().map(|g| g.compile_matcher()).collect();
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ChannelConfig {
    // The path to write feed pages to relative to the target
    // build directory, eg: `posts`.
    //
    // Feed files will be placed in this directory.
    pub target: Option<PathBuf>,

    // List of file types to generate for this feed
    pub types: Vec<FeedType>,

    // Configuration options for indexing behavior
    pub includes: Vec<Glob>,
    pub excludes: Vec<Glob>,

    #[serde(skip)]
    pub include_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub exclude_match: Vec<GlobMatcher>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            target: None,
            types: vec![FeedType::Json, FeedType::Rss],
            includes: Vec::new(),
            excludes: Vec::new(),
            include_match: Vec::new(),
            exclude_match: Vec::new(),
        }
    }
}

impl ChannelConfig {
    pub fn filter(&self, href: &str) -> bool {
        for glob in self.exclude_match.iter() {
            if glob.is_match(href) { return false; }
        }
        if self.include_match.is_empty() { return true; }
        for glob in self.include_match.iter() {
            if glob.is_match(href) { return true; }
        }
        false
    }
}
