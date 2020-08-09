use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};

use super::scores::*;
use crate::common::{Fields, IndexFromFile};
use crate::config::TitleBoost;

pub type EntryIndex = usize;
pub type AliasTarget = String;
pub type Score = u8;

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct QueryOptions {
    // Total number of results to return
    pub results: usize,
    // Boost for title query matches
    pub title_boost: TitleBoost,
    // Number of excerpts to buffer
    pub excerpt_buffer: u8,
    // Maximum number of excerpts per result
    pub excerpts_per_result: u8,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            excerpt_buffer: 8,
            excerpts_per_result: 5,
            results: 10,
            title_boost: Default::default(),
        }
    }
}

#[wasm_bindgen]
impl QueryOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }
}

/**
 * A serialized Index, for all intents and purposes, is the whole contents of
 * a Stork index file.
 */
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Index {
    pub entries: Vec<Entry>,
    pub containers: HashMap<String, Container>,
}

impl TryFrom<&IndexFromFile> for Index {
    type Error = rmp_serde::decode::Error;
    fn try_from(file: &IndexFromFile) -> std::result::Result<Self, Self::Error> {
        let (version_size_bytes, rest) = file.split_at(std::mem::size_of::<u64>());
        let version_size = u64::from_be_bytes(version_size_bytes.try_into().unwrap());
        let (_version_bytes, rest) = rest.split_at(version_size as usize);

        let (index_size_bytes, rest) = rest.split_at(std::mem::size_of::<u64>());
        let index_size = u64::from_be_bytes(index_size_bytes.try_into().unwrap());
        let (index_bytes, _rest) = rest.split_at(index_size as usize);

        rmp_serde::from_read_ref(index_bytes)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub contents: String,
    pub title: String,
    pub url: String,
    pub fields: Fields,
}

/**
 * A Container holds:
 *
 * - a HashMap of EntryIndexes to SearchResults
 * - a HashMap of AliasTargets to scores
 *
 * Each valid query should return a single Container. It is possible to derive
 * all search results for a given query from a single container.
 */
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Container {
    // #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub results: HashMap<EntryIndex, SearchResult>,

    // #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aliases: HashMap<AliasTarget, Score>,
}

impl Container {
    pub fn new() -> Container {
        Container::default()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchResult {
    pub excerpts: Vec<Excerpt>,
    pub score: Score,
}

impl SearchResult {
    pub fn new() -> SearchResult {
        SearchResult {
            excerpts: vec![],
            score: MATCHED_WORD_SCORE,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Excerpt {
    pub word_index: usize,

    // #[serde(default, skip_serializing_if = "WordListSource::is_default")]
    pub source: WordListSource,

    // #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: Fields,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordListSource {
    Title,
    Contents,
}

impl Default for WordListSource {
    fn default() -> Self {
        WordListSource::Contents
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AnnotatedWord {
    pub word: String,
    pub fields: Fields,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contents {
    pub word_list: Vec<AnnotatedWord>,
}

impl Contents {
    pub fn get_full_text(&self) -> String {
        self.word_list
            .iter()
            .map(|aw| aw.word.clone())
            .collect::<Vec<String>>()
            .join(" ")
        // encode_minimal(out.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;
    use std::fs;
    use std::io::{BufReader, Read};

    #[test]
    fn can_parse_0_7_0_index() {
        let file = fs::File::open("./test/assets/federalist-min.st").unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut index_bytes: Vec<u8> = Vec::new();
        let _bytes_read = buf_reader.read_to_end(&mut index_bytes);
        let index = Index::try_from(index_bytes.as_slice()).unwrap();
        assert_eq!(1, index.entries.len());
        assert_eq!(2456, index.containers.len());
    }

    #[test]
    fn get_full_text() {
        let intended = "This is-a set of words.".to_string();
        let generated = Contents {
            word_list: vec![
                AnnotatedWord {
                    word: "This".to_string(),
                    ..Default::default()
                },
                AnnotatedWord {
                    word: "is-a".to_string(),
                    fields: HashMap::default(),
                },
                AnnotatedWord {
                    word: "set".to_string(),
                    ..Default::default()
                },
                AnnotatedWord {
                    word: "of".to_string(),
                    ..Default::default()
                },
                AnnotatedWord {
                    word: "words.".to_string(),
                    ..Default::default()
                },
            ],
        }
        .get_full_text();

        assert_eq!(intended, generated);
    }
}
