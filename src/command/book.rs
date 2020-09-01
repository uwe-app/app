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

    let spaces = workspace::find(&options.project, true)?;
    if spaces.has_multiple_projects() {
        return Err(Error::BookCreateWorkspace);
    }

    book::add(
        spaces.into_iter().take(1).next().as_ref().unwrap(),
        options.path.unwrap(),
        options.title,
        options.authors,
    )?;

    Ok(())
}

pub fn list(options: BookOptions) -> Result<()> {
    let spaces = workspace::find(&options.project, true)?;
    for config in spaces.into_iter() {
        book::list(&config)?;
    }
    Ok(())
}

pub fn build(options: BookOptions) -> Result<()> {
    let spaces = workspace::find(&options.project, true)?;
    for config in spaces.into_iter() {
        // TODO: support release flag!
        book::build(&config, options.target.clone(), false)?;
    }
    Ok(())
}
