use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::Error;

// SEE: https://developer.mozilla.org/en-US/docs/Web/HTML/Link_types

static ALTERNATE: &str = "alternate";
static AUTHOR: &str = "author";
static BOOKMARK: &str = "bookmark";
static CANONICAL: &str = "canonical";
static EXTERNAL: &str = "external";
static HELP: &str = "help";
static ICON: &str = "icon";
static LICENSE: &str = "license";
static MANIFEST: &str = "manifest";
static MODULE_PRELOAD: &str = "modulepreload";
static NEXT: &str = "next";
static NO_FOLLOW: &str = "nofollow";
static NO_OPENER: &str = "noopener";
static NO_REFERRER: &str = "noreferrer";
static PING_BACK: &str = "pingback";
static PRE_FETCH: &str = "prefetch";
static PRE_LOAD: &str = "preload";
static PREV: &str = "prev";
static SEARCH: &str = "search";
static SHORT_LINK: &str = "shortlink";
static STYLE_SHEET: &str = "stylesheet";
static TAG: &str = "tag";

static ANONYMOUS: &str = "anonymous";
static USE_CREDENTIALS: &str = "use-credentials";

static AUDIO: &str = "audio";
static DOCUMENT: &str = "document";
static EMBED: &str = "embed";
static FETCH: &str = "fetch";
static FONT: &str = "font";
static IMAGE: &str = "image";
static OBJECT: &str = "object";
static SCRIPT: &str = "script";
static STYLE: &str = "style";
static TRACK: &str = "track";
static VIDEO: &str = "video";
static WORKER: &str = "worker";

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RelValue {
    Alternate,
    Author,
    Bookmark,
    Canonical,
    // TODO: dns-prefetch - maybe later?
    External,
    Help,
    Icon,
    License,
    Manifest,
    ModulePreload,
    Next,
    NoFollow,
    NoOpener,
    NoReferrer,
    PingBack,
    PreFetch,
    PreLoad,
    Prev,
    Search,
    ShortLink,
    StyleSheet,
    Tag,
}

impl FromStr for RelValue {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let matched = if s == ALTERNATE {
            Self::Alternate
        } else if s == AUTHOR {
            Self::Author
        } else if s == BOOKMARK {
            Self::Bookmark
        } else if s == CANONICAL {
            Self::Canonical
        } else if s == EXTERNAL {
            Self::External
        } else if s == HELP {
            Self::Help
        } else if s == ICON {
            Self::Icon
        } else if s == LICENSE {
            Self::License
        } else if s == MANIFEST {
            Self::Manifest
        } else if s == MODULE_PRELOAD {
            Self::ModulePreload
        } else if s == NEXT {
            Self::Next
        } else if s == NO_FOLLOW {
            Self::NoFollow
        } else if s == NO_OPENER {
            Self::NoOpener
        } else if s == NO_REFERRER {
            Self::NoReferrer
        } else if s == PING_BACK {
            Self::PingBack
        } else if s == PRE_FETCH {
            Self::PreFetch
        } else if s == PRE_LOAD {
            Self::PreLoad
        } else if s == PREV {
            Self::Prev
        } else if s == SEARCH {
            Self::Search
        } else if s == SHORT_LINK {
            Self::ShortLink
        } else if s == STYLE_SHEET {
            Self::StyleSheet
        } else if s == TAG {
            Self::Tag
        } else {
            return Err(Error::InvalidRelValue(s.to_string()));
        };
        Ok(matched)
    }
}

impl RelValue {
    pub fn as_str(&self) -> &str {
        match *self {
            Self::Alternate => ALTERNATE,
            Self::Author => AUTHOR,
            Self::Bookmark => BOOKMARK,
            Self::Canonical => CANONICAL,
            Self::External => EXTERNAL,
            Self::Help => HELP,
            Self::Icon => ICON,
            Self::License => LICENSE,
            Self::Manifest => MANIFEST,
            Self::ModulePreload => MODULE_PRELOAD,
            Self::Next => NEXT,
            Self::NoFollow => NO_FOLLOW,
            Self::NoOpener => NO_OPENER,
            Self::NoReferrer => NO_REFERRER,
            Self::PingBack => PING_BACK,
            Self::PreFetch => PRE_FETCH,
            Self::PreLoad => PRE_LOAD,
            Self::Prev => PREV,
            Self::Search => SEARCH,
            Self::ShortLink => SHORT_LINK,
            Self::StyleSheet => STYLE_SHEET,
            Self::Tag => TAG,
        }
    }
}

impl fmt::Display for RelValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum CrossOrigin {
    Anonymous,
    UseCredentials,
}

impl CrossOrigin {
    pub fn as_str(&self) -> &str {
        match *self {
            Self::Anonymous => ANONYMOUS,
            Self::UseCredentials => USE_CREDENTIALS,
        }
    }
}

impl fmt::Display for CrossOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum As {
    Audio,
    Document,
    Embed,
    Fetch,
    Font,
    Image,
    Object,
    Script,
    Style,
    Track,
    Video,
    Worker,
}

impl As {
    pub fn as_str(&self) -> &str {
        match *self {
            Self::Audio => AUDIO,
            Self::Document => DOCUMENT,
            Self::Embed => EMBED,
            Self::Fetch => FETCH,
            Self::Font => FONT,
            Self::Image => IMAGE,
            Self::Object => OBJECT,
            Self::Script => SCRIPT,
            Self::Style => STYLE,
            Self::Track => TRACK,
            Self::Video => VIDEO,
            Self::Worker => WORKER,
        }
    }
}

impl fmt::Display for As {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub mod referrer_policy {

    use serde::{Deserialize, Serialize};
    use std::fmt;

    static NO_REFERRER: &str = "no-referrer";
    static NO_REFERRER_WHEN_DOWNGRADE: &str = "no-referrer-when-downgrade";
    static ORIGIN: &str = "origin";
    static ORIGIN_WHEN_CROSS_ORIGIN: &str = "origin-when-cross-origin";
    static SAME_ORIGIN: &str = "same-origin";
    static STRICT_ORIGIN: &str = "strict-origin";
    static STRICT_ORIGIN_WHEN_CROSS_ORIGIN: &str =
        "strict-origin-when-cross-origin";

    #[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
    #[serde(rename_all = "kebab-case")]
    pub enum ReferrerPolicy {
        NoReferrer,
        NoReferrerWhenDowngrade,
        Origin,
        OriginWhenCrossOrigin,
        SameOrigin,
        StrictOrigin,
        StrictOriginWhenCrossOrigin,
        // NOTE: there is also unsafe-url but we prefer to avoid unsafe
    }

    impl ReferrerPolicy {
        pub fn as_str(&self) -> &str {
            match *self {
                Self::NoReferrer => NO_REFERRER,
                Self::NoReferrerWhenDowngrade => NO_REFERRER_WHEN_DOWNGRADE,
                Self::Origin => ORIGIN,
                Self::OriginWhenCrossOrigin => ORIGIN_WHEN_CROSS_ORIGIN,
                Self::SameOrigin => SAME_ORIGIN,
                Self::StrictOrigin => STRICT_ORIGIN,
                Self::StrictOriginWhenCrossOrigin => {
                    STRICT_ORIGIN_WHEN_CROSS_ORIGIN
                }
            }
        }
    }

    impl fmt::Display for ReferrerPolicy {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.as_str())
        }
    }

    impl Default for ReferrerPolicy {
        fn default() -> Self {
            Self::NoReferrerWhenDowngrade
        }
    }
}
