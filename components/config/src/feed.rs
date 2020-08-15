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
    pub fn get_name(&self) -> String {
        format!("{}.{}", self.get_file_name(), self.get_extension())
    }

    pub fn get_file_name(&self) -> &str {
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
pub struct FeedTemplates {
    pub json: Option<PathBuf>,
    pub rss: Option<PathBuf>,
    pub atom: Option<PathBuf>,
}

impl FeedTemplates {
    pub fn get(&self, feed_type: &FeedType) -> &Option<PathBuf> {
        match *feed_type {
            FeedType::Json => &self.json,
            FeedType::Rss => &self.rss,
            FeedType::Atom => &self.atom,
        } 
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct FeedConfig {
    // The limit for the number of items in each feed.
    //
    // The resulting list will be truncated to this value.
    pub limit: Option<usize>,

    // List of custom templates to use for each feed.
    //
    // When specified they override the default templates 
    // for each feed type.
    pub templates: FeedTemplates,

    #[serde(flatten)]
    pub channels: HashMap<String, ChannelConfig>,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            limit: Some(100),
            templates: Default::default(),
            channels: HashMap::new(),
        }
    }
}

impl FeedConfig {
    // Prepare the configuration by compiling the glob matchers.
    pub fn prepare(&mut self) {
        for (k, v) in self.channels.iter_mut() {
            v.target = Some(k.to_string());
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
    pub target: Option<String>,

    // A title for the feed channel.
    pub title: Option<String>,

    // A description for the feed channel.
    pub description: Option<String>,

    // Path for a favicon, it will be made absolute.
    pub favicon: Option<String>,

    // Path for an icon, it will be made absolute.
    pub icon: Option<String>,

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
            title: None,
            description: None,
            favicon: None,
            icon: None,
            types: vec![FeedType::Json, FeedType::Rss, FeedType::Atom],
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
