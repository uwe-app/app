use std::path::PathBuf;

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
        return Err(Error::BookCreatePath);
    }

    let spaces = workspace::find(&options.project, true)?.flatten();
    if spaces.len() != 1 {
        return Err(Error::BookCreateWorkspace);
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
    let spaces = workspace::find(&options.project, true)?.flatten();
    for config in spaces {
        book::list(&config)?;
    }
    Ok(())
}

pub fn build(options: BookOptions) -> Result<()> {
    let spaces = workspace::find(&options.project, true)?.flatten();
    for config in spaces {
        // TODO: support release flag!
        book::build(&config, options.target.clone(), false)?;
    }
    Ok(())
}
