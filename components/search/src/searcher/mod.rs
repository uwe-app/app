pub mod index_analyzer;

use crate::common::Fields;
use crate::index::v1;
use crate::index::v1::structs::{Index, QueryOptions};
use index_analyzer::{IndexVersion, VersionParseError};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct SearchOutput {
    pub results: Vec<OutputResult>,
    pub total_hit_count: usize,
    pub url_prefix: String,
}

/**
 * Correlates an OutputEntry with a vector of excerpts. Represents a single
 * document that contains search results.
 */
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OutputResult {
    pub entry: OutputEntry,
    pub excerpts: Vec<Excerpt>,
    pub title_highlight_ranges: Vec<HighlightRange>,
    pub score: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OutputEntry {
    pub url: String,
    pub title: String,
    pub fields: Fields,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Excerpt {
    pub text: String,
    pub highlight_ranges: Vec<HighlightRange>,
    pub score: usize,
    pub fields: Fields,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HighlightRange {
    pub beginning: usize,
    pub end: usize,
}

#[derive(Debug)]
pub enum SearchError {
    /// If version can't be parsed when reading the index
    VersionParseError(VersionParseError),

    /// If the index deserialization returns an error (applicable to v1 only)
    IndexParseError,

    // If the JSON serialization engine crashes while turning the SearchOutput
    // into a string
    JSONSerializationError,
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc: String = match self {
            SearchError::VersionParseError(e) => format!("{}", e),
            SearchError::IndexParseError => "Could not parse index file.".to_string(),
            SearchError::JSONSerializationError => "Could not format search results.".to_string(),
        };

        write!(f, "{}", desc)
    }
}

pub fn search(
    index: &Index,
    version: &IndexVersion,
    query: &str,
    options: &QueryOptions,
) -> Result<SearchOutput, SearchError> {
    let search_function = match version {
        IndexVersion::V1 => v1::search::search,
    };

    search_function(index, query, options)
}
