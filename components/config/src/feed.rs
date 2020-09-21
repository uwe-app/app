use std::fmt;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils::matcher::GlobPatternMatcher;

static JSON: &str = "json";
static XML: &str = "xml";

static JSON_MIME: &str = "application/feed+json";
static ATOM_MIME: &str = "application/atom+xml";
static RSS_MIME: &str = "application/rss+xml";

static JSON_NAME: &str = "feed";
static RSS_NAME: &str = "rss";
static ATOM_NAME: &str = "atom";

static PLUGIN_NAME: &str = "std::feed";

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum FeedType {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "rss")]
    Rss,
    #[serde(rename = "atom")]
    Atom,
}

impl fmt::Display for FeedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_file_name())
    }
}

impl FeedType {
    pub fn get_name(&self) -> String {
        format!("{}.{}", self.get_file_name(), self.get_extension())
    }

    pub fn get_mime(&self) -> &str {
        match *self {
            Self::Json => JSON_MIME,
            Self::Rss => RSS_MIME,
            Self::Atom => ATOM_MIME,
        }
    }

    pub fn get_file_name(&self) -> &str {
        match *self {
            Self::Json => JSON_NAME,
            Self::Rss => RSS_NAME,
            Self::Atom => ATOM_NAME,
        }
    }

    pub fn get_extension(&self) -> &str {
        match *self {
            Self::Json => JSON,
            _ => XML,
        }
    }
}

// The partial names in the feed template plugin.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct FeedTemplateNames {
    pub json: Option<String>,
    pub rss: Option<String>,
    pub atom: Option<String>,
}

impl Default for FeedTemplateNames {
    fn default() -> Self {
        Self {
            json: Some(JSON_NAME.to_string()),
            rss: Some(RSS_NAME.to_string()),
            atom: Some(ATOM_NAME.to_string()),
        }
    }
}

impl FeedTemplateNames {
    pub fn get(&self, feed_type: &FeedType) -> &Option<String> {
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

    // The name of the plugin defining the feed template partials.
    pub plugin: Option<String>,

    // List of custom template names to use for each feed.
    //
    // When specified they override the default templates
    // for each feed type.
    pub names: FeedTemplateNames,

    #[serde(flatten)]
    pub channels: HashMap<String, ChannelConfig>,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            limit: Some(100),
            plugin: Some(PLUGIN_NAME.to_string()),
            names: Default::default(),
            channels: HashMap::new(),
        }
    }
}

impl FeedConfig {
    // Prepare the configuration by compiling the glob matchers.
    pub(crate) fn prepare(&mut self) {
        for (k, v) in self.channels.iter_mut() {
            v.target = Some(k.to_string());
            v.matcher.compile();
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

    #[serde(flatten)]
    pub matcher: GlobPatternMatcher,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            target: Some("".to_string()),
            title: None,
            description: None,
            favicon: None,
            icon: None,
            types: vec![FeedType::Json, FeedType::Rss, FeedType::Atom],
            matcher: Default::default(),
        }
    }
}
