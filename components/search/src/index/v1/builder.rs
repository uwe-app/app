use super::scores::*;
use super::structs::*;
use crate::config::DataSource;
use crate::config::{Config, StemmingConfig};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::common::Fields;

extern crate rust_stemmers;
use rust_stemmers::{Algorithm, Stemmer};

#[derive(Debug)]
pub struct IntermediateEntry {
    pub contents: Contents,
    pub stem_algorithm: Option<Algorithm>,
    pub title: String,
    pub url: String,
    pub fields: Fields,
}

impl From<&IntermediateEntry> for Entry {
    fn from(ie: &IntermediateEntry) -> Self {
        Entry {
            contents: ie.contents.get_full_text(),
            title: ie.title.clone(),
            url: ie.url.clone(),
            fields: ie.fields.clone(),
        }
    }
}

pub fn intermediate(
    buffer: &str,
    title: &str,
    url: &str,
    fields: Fields,
) -> IntermediateEntry {
    // FIXME
    let stem_algorithm = Some(Algorithm::English);

    let contents = get_text_contents(buffer);
    IntermediateEntry {
        contents,
        stem_algorithm,
        title: title.to_string(),
        url: url.to_string(),
        fields: fields.clone(),
    }
}

fn get_text_contents(buffer: &str) -> Contents {
    Contents {
        word_list: buffer
            .split_whitespace()
            .map(|word| AnnotatedWord {
                word: word.to_string(),
                ..Default::default()
            })
            .collect(),
    }
}

pub fn build(config: &Config) -> Index {
    let mut intermediate_entries: Vec<IntermediateEntry> = Vec::new();
    //let mut containers: HashMap<String, Container> = HashMap::new();

    // Step 1: Fill entries vector
    let base_directory = Path::new(&config.input.base_directory);
    for file in config.input.files.iter() {
        let buffer: String = match &file.source {
            DataSource::Contents(contents) => contents.to_string(),
            DataSource::FilePath(path_string) => {
                let full_pathname = &base_directory.join(&path_string);
                let file = File::open(&full_pathname).unwrap();
                let mut buf_reader = BufReader::new(file);
                let mut buffer = String::new();
                let _bytes_read = buf_reader.read_to_string(&mut buffer);
                buffer
            }
        };

        let current_stem_config = file
            .stemming_override
            .clone()
            .unwrap_or_else(|| config.input.stemming.clone());

        let stem_algorithm: Option<Algorithm> = match current_stem_config {
            StemmingConfig::Language(alg) => Some(alg.to_owned()),
            StemmingConfig::None => None,
        };

        let contents: Contents = get_text_contents(&buffer);
        //get_word_list_generator(filetype)
        //.create_word_list(&config.input, &buffer);

        let entry = IntermediateEntry {
            contents,
            stem_algorithm,
            title: file.title.clone(),
            url: file.url.clone(),
            fields: file.fields.clone(),
        };

        intermediate_entries.push(entry);
    }

    compile(intermediate_entries)
}

pub fn compile(intermediates: Vec<IntermediateEntry>) -> Index {
    let mut idx: Index = Default::default();

    let mut containers: HashMap<String, Container> = HashMap::new();
    let mut stems: HashMap<String, Vec<String>> = HashMap::new();

    for entry in &intermediates {
        let contents = &entry.contents;

        if let Some(stem_algorithm) = entry.stem_algorithm {
            for annotated_word in contents.word_list.iter() {
                let normalized_word = remove_surrounding_punctuation(
                    &annotated_word.word.to_lowercase(),
                );
                let stem = Stemmer::create(stem_algorithm)
                    .stem(&normalized_word)
                    .to_string();
                let stem_vector =
                    stems.entry(stem).or_insert_with(Vec::default);
                if !stem_vector.contains(&normalized_word) {
                    stem_vector.push(normalized_word);
                }
            }
        }
    }

    // Step 2: Fill containers map
    for (entry_index, entry) in intermediates.iter().enumerate() {
        let words_in_title: Vec<AnnotatedWord> = entry
            .title
            .split_whitespace()
            .map(|w| AnnotatedWord {
                word: w.to_string(),
                ..Default::default()
            })
            .collect();

        let words_in_contents: Vec<AnnotatedWord> =
            entry.contents.word_list.to_owned();

        let word_lists = vec![
            (WordListSource::Title, words_in_title),
            (WordListSource::Contents, words_in_contents),
        ];

        for (source, word_list) in word_lists {
            for (word_index, annotated_word) in word_list.iter().enumerate() {
                let normalized_word = remove_surrounding_punctuation(
                    &annotated_word.word.to_lowercase(),
                );
                if normalized_word.is_empty() {
                    break;
                }

                // Step 2A: Fill the container's results map
                let results_map = &mut containers
                    .entry(normalized_word.clone())
                    .or_insert_with(Container::new)
                    .results;

                let entry_result: &mut SearchResult = results_map
                    .entry(entry_index)
                    .or_insert_with(SearchResult::new);

                entry_result.excerpts.push(Excerpt {
                    word_index,
                    source,
                    fields: annotated_word.fields.clone(),
                });

                // Step 2B: Fill _other containers'_ aliases maps with the
                // prefixes of this word
                let chars: Vec<char> = normalized_word.chars().collect();
                for n in 3..chars.len() {
                    let substring: String = chars[0..n].iter().collect();

                    let alises_map = &mut containers
                        .entry(substring.clone())
                        .or_insert_with(Container::new)
                        .aliases;

                    let _alias_score = alises_map
                        .entry(normalized_word.clone())
                        .or_insert(PREFIX_SCORE - (chars.len() - n) as u8);
                }

                // Step 2C: Fill _other containers'_ alias maps with the
                // reverse-stems of this word
                if let Some(stem_algorithm) = entry.stem_algorithm {
                    let stem = Stemmer::create(stem_algorithm)
                        .stem(&normalized_word)
                        .to_string();
                    if let Some(reverse_stems_vector) = stems.get(&stem) {
                        for reverse_stem in reverse_stems_vector {
                            if reverse_stem != &normalized_word {
                                let _alias_score = containers
                                    .entry(reverse_stem.clone())
                                    .or_insert_with(Container::new)
                                    .aliases
                                    .entry(normalized_word.clone())
                                    .or_insert(STEM_SCORE as u8);
                            }
                        }
                    }
                }
            }
        }
    }

    let entries: Vec<Entry> = intermediates.iter().map(Entry::from).collect();
    idx.containers = containers;
    idx.entries = entries;
    idx
}

fn remove_surrounding_punctuation(input: &str) -> String {
    let mut chars: Vec<char> = input.chars().collect();

    while chars.first().unwrap_or(&'a').is_ascii_punctuation() {
        chars.remove(0);
    }

    while chars.last().unwrap_or(&'a').is_ascii_punctuation() {
        chars.pop();
    }

    chars.into_iter().collect()
}
