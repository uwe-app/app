use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GlobPatternMatcher {
    // Configuration options for indexing behavior
    pub includes: Vec<Glob>,
    pub excludes: Vec<Glob>,

    #[serde(skip)]
    pub include_match: Vec<GlobMatcher>,
    #[serde(skip)]
    pub exclude_match: Vec<GlobMatcher>,
}

impl Default for GlobPatternMatcher {
    fn default() -> Self {
        Self {
            includes: Vec::new(),
            excludes: Vec::new(),
            include_match: Vec::new(),
            exclude_match: Vec::new(),
        }
    }
}

impl GlobPatternMatcher {
    /// Compile the glob matchers.
    ///
    /// Callers should ensure this is done early, eg, when
    /// the configuration data has been loaded.
    pub fn compile(&mut self) {
        self.include_match = self.includes.iter().map(|g| g.compile_matcher()).collect();
        self.exclude_match = self.excludes.iter().map(|g| g.compile_matcher()).collect();
    }

    /// Determine if a pattern matches.
    ///
    /// No assumptions are made about matching when the
    /// pattern lists are empty.
    pub fn matches(&self, href: &str) -> bool {
        self.test(href, false)
    }

    /// Determine if a pattern should be filtered.
    ///
    /// If the include list is empty it is assumed the
    /// pattern matches. Excludes take precedence.
    pub fn filter(&self, href: &str) -> bool {
        self.test(href, true)
    }

    fn test(&self, href: &str, empty: bool) -> bool {
        for glob in self.exclude_match.iter() {
            if glob.is_match(href) {
                return false;
            }
        }
        if empty && self.include_match.is_empty() {
            return true;
        }
        for glob in self.include_match.iter() {
            if glob.is_match(href) {
                return true;
            }
        }
        false
    }
}
