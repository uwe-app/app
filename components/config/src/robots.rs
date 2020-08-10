use std::fmt;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;

static WILDCARD: &str = "*";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct RobotsConfig {
    #[serde(flatten)]
    pub rules: HashMap<String, RobotsRule>,
    #[serde(skip)]
    pub sitemaps: Vec<Url>,
}

impl Default for RobotsConfig {
    fn default() -> Self {
        let mut rules = HashMap::new();
        let rule = RobotsRule::all();
        rules.insert(WILDCARD.to_string(), rule);
        Self { rules, sitemaps: vec![] }
    }
}

impl fmt::Display for RobotsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (ua, rule) in self.rules.iter() {
            write!(f, "user-agent: {}\n", ua)?;
            rule.fmt(f)?;
        }

        for url in self.sitemaps.iter() {
            write!(f, "sitemap: {}\n", url.to_string())?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RobotsRule {
    pub allow: Option<Vec<String>>,
    pub disallow: Option<Vec<String>>,
}

impl RobotsRule {
    pub fn all() -> Self {
        Self {
            allow: Some(vec![WILDCARD.to_string()]),
            disallow: None,
        }
    }
}

impl fmt::Display for RobotsRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref allow) = self.allow {
            for path in allow.iter() { write!(f, "allow: {}\n", path)?; }
        }
        if let Some(ref disallow) = self.disallow {
            for path in disallow.iter() { write!(f, "disallow: {}\n", path)?; }
        }
        Ok(())
    }
}
