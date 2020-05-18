use std::path::PathBuf;

use regex::Regex;

mod build;
mod fs;
mod matcher;
mod parser;
mod template;

use matcher::FileMatcher;
use build::Builder;

pub struct Options {
    pub source: PathBuf,
    pub target: PathBuf,
    pub follow_links: bool,
    pub exclude: Option<Vec<Regex>>,
    pub layout: String,
    pub template: String,
    pub theme: String,
    pub clean: bool,
    pub minify: bool,
}

pub fn build(options: Options) {
    let matcher = FileMatcher::new(&options);
    let finder = Builder::new(&matcher, &options);
    finder.run();
}

