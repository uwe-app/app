use std::path::PathBuf;

use config::Config;

use crate::{Error, Result};

#[derive(Debug, Default)]
pub struct BookOptions {
    pub project: PathBuf,
    pub target: Vec<PathBuf>,

    // For new books
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    pub authors: Option<Vec<String>>,
}

pub fn add(options: BookOptions) -> Result<()> {
    if options.path.is_none() {
        return Err(Error::new(format!("Book creation requires a path")));
    }

    let mut spaces: Vec<Config> = Vec::new();
    workspace::find(&options.project, true, &mut spaces)?;
    if spaces.len() != 1 {
        return Err(Error::new(format!(
            "Book creation requires a project not a workspace"
        )));
    }

    book::add(
        &spaces[0],
        options.path.unwrap(),
        options.title,
        options.authors,
    )?;

    Ok(())
}

pub fn list(options: BookOptions) -> Result<()> {
    let mut spaces: Vec<Config> = Vec::new();
    workspace::find(&options.project, true, &mut spaces)?;
    for config in spaces {
        book::list(&config)?;
    }
    Ok(())
}

pub fn build(options: BookOptions) -> Result<()> {
    let mut spaces: Vec<Config> = Vec::new();
    workspace::find(&options.project, true, &mut spaces)?;
    for config in spaces {
        // TODO: support release flag!
        book::build(&config, options.target.clone(), false)?;
    }
    Ok(())
}
