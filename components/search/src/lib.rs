pub mod common;
pub mod config;
pub mod index;
pub mod searcher;

use thiserror::Error;
use serde::{Deserialize, Serialize};

use common::IndexFromFile;
use config::Config;
use searcher::index_analyzer::{parse_index_version, IndexVersion};
use searcher::SearchError;

use std::convert::TryFrom;

use index::v1 as LatestVersion;
pub use LatestVersion::*;


#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Rmp(#[from] rmp_serde::encode::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchIndexInfo {
    pub name: String,
    pub size: usize,
    pub version: String,
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct SearchIndex {
    file: Vec<u8>,
    index: Index,
    version: IndexVersion,
    options: QueryOptions,
}

#[wasm_bindgen]
impl SearchIndex {
    #[wasm_bindgen(constructor)]
    pub fn new(file: &IndexFromFile, opts: String) -> Self {
        console_error_panic_hook::set_once();

        let options: QueryOptions =
            serde_json::from_str(&opts).unwrap_or(Default::default());

        // FIXME: handle errors gracefully
        let version = parse_index_version(file).unwrap();
        let index = Index::try_from(file).unwrap();

        Self {
            file: file.to_vec(),
            version,
            index,
            options,
        }
    }

    pub fn print(&self, name: String) {
        let info = SearchIndexInfo {
            name,
            size: self.file.len(),
            version: self.version.to_string(),
        };
        let msg = serde_json::to_string(&info).unwrap();
        log(&msg);
    }

    pub fn search(&self, query: String) -> String {
        log(&format!("Index search is running with query {}", query));

        let search_result = search(&self.file, query, &self.options).and_then(|output| {
            serde_json::to_string(&output).map_err(|_e| SearchError::JSONSerializationError)
        });

        match search_result {
            Ok(res) => res,
            Err(e) => format!("{{error: '{}'}}", e),
        }
    }
}

pub fn search(
    index: &IndexFromFile,
    query: String,
    options: &QueryOptions,
) -> std::result::Result<searcher::SearchOutput, searcher::SearchError> {
    searcher::search(index, query.as_str(), options)
}

pub fn build(config: &Config) -> Index {
    builder::build(config)
}
