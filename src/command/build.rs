use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::build::Builder;
use crate::build::loader;
use crate::Error;

#[derive(Debug, Serialize, Deserialize)]
pub enum BuildTag {
    Custom(String),
    Debug,
    Release
}

impl BuildTag {
    pub fn get_path_name(&self) -> String {
        match self {
            BuildTag::Debug => return "debug".to_owned(),
            BuildTag::Release => return "release".to_owned(),
            BuildTag::Custom(s) => return s.to_owned()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildOptions {
    pub source: PathBuf,
    pub output: PathBuf,
    pub target: PathBuf,
    pub directory: Option<PathBuf>,
    pub max_depth: Option<usize>,
    pub release: bool,
    pub follow_links: bool,
    pub strict: bool,
    pub clean_url: bool,
    pub tag: BuildTag,
}

pub fn build(options: BuildOptions) -> Result<(), Error> {
    if let Err(e) = loader::load(&options) {
        return Err(e)
    }

    //let test = Path::new("site/index.md");
    //println!("{:?}", loader::compute(test));

    let mut builder = Builder::new(&options);
    builder.build()
}

