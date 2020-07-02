use std::path::PathBuf;

use config::Config;

use crate::Result;

#[derive(Debug, Default)]
pub struct BookOptions {
    pub project: PathBuf,
    pub target: Vec<PathBuf>,
}

pub fn add(options: BookOptions) -> Result<()> {
    println!("TODO: add a new book!");
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
