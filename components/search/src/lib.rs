pub mod common;
pub mod config;
pub mod index;
pub mod searcher;

use thiserror::Error;

use common::IndexFromFile;
use config::Config;
use searcher::index_analyzer::parse_index_version;
use searcher::SearchError;

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

#[wasm_bindgen]
pub fn wasm_search(index: &IndexFromFile, query: String, options: String) -> String {
    console_error_panic_hook::set_once();
    let opts: QueryOptions = serde_json::from_str(&options).unwrap_or(Default::default());
    let search_result = search(index, query, &opts).and_then(|output| {
        serde_json::to_string(&output).map_err(|_e| SearchError::JSONSerializationError)
    });

    match search_result {
        Ok(string) => string,
        Err(e) => format!("{{error: '{}'}}", e),
    }
}

#[wasm_bindgen]
pub fn get_index_version(index: &IndexFromFile) -> String {
    let parse_result = parse_index_version(index);

    match parse_result {
        Ok(v) => format!("{}", v),
        Err(e) => format!("{}", e),
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
