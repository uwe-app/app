use std::path::Path;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use serde::{Deserialize, Serialize};

use super::scores::*;
use crate::common::{Fields, InternalWordAnnotation, IndexFromFile};
use crate::config::TitleBoost;

use crate::Result;

pub type EntryIndex = usize;
pub type AliasTarget = String;
pub type Score = u8;

/**
 * A serialized Index, for all intents and purposes, is the whole contents of
 * a Stork index file.
 */
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Index {
    pub config: PassthroughConfig,
    pub entries: Vec<Entry>,
    pub containers: HashMap<String, Container>,
}

impl Index {
    pub fn write<P: AsRef<Path>>(&self, filename: P, debug: bool) -> Result<usize> {
        let file = File::create(filename)?;
        let mut bufwriter = BufWriter::new(file);
        let write_version = super::VERSION_STRING.as_bytes();
        if debug {
            self.write_debug(&mut bufwriter, &write_version)
        } else {
            self.write_release(&mut bufwriter, &write_version)
        }
    }

    fn write_release(&self, bufwriter: &mut BufWriter<File>, write_version: &[u8]) -> Result<usize> {
        let mut bytes_written: usize = 0;
        let index_bytes = rmp_serde::to_vec(self)?;
        let byte_vectors_to_write = [write_version, index_bytes.as_slice()];
        for vec in byte_vectors_to_write.iter() {
            bytes_written += bufwriter.write(&(vec.len() as u64).to_be_bytes())?;
            bytes_written += bufwriter.write(vec)?;
        }
        Ok(bytes_written)
    }

    fn write_debug(&self, bufwriter: &mut BufWriter<File>, write_version: &[u8]) -> Result<usize> {
        let index_serialized = serde_json::to_string_pretty(self)?;

        let byte_vectors_to_write = [write_version, index_serialized.as_bytes()];

        for vec in byte_vectors_to_write.iter() {
            let _ = bufwriter.write(vec.len().to_string().as_bytes());
            let _ = bufwriter.write(b"\n");
            let _ = bufwriter.write(vec);
            let _ = bufwriter.write(b"\n\n");
        }

        // Return zero bytes written so that the frontend can alert the user
        // when they write an index in debug mode
        Ok(0)
    }
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
pub struct PassthroughConfig {
    pub url_prefix: String,
    pub title_boost: TitleBoost,
    pub excerpt_buffer: u8,
    pub excerpts_per_result: u8,
    pub displayed_results_count: u8,
}


impl Default for PassthroughConfig {
    fn default() -> Self {
        Self {
            url_prefix: "".to_string(),
            title_boost: Default::default(),
            excerpt_buffer: 8,
            excerpts_per_result: 5,
            displayed_results_count: 10,
        }
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

    // #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_annotations: Vec<InternalWordAnnotation>,

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

// impl WordListSource {
//     #[allow(clippy::trivially_copy_pass_by_ref)]
//     fn is_default(&self) -> bool {
//         self == &WordListSource::default()
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AnnotatedWord {
    pub word: String,
    pub internal_annotations: Vec<InternalWordAnnotation>,
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
                    internal_annotations: vec![InternalWordAnnotation::SRTUrlSuffix(
                        "a".to_string(),
                    )],
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
