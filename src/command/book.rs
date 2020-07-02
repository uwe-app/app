use std::path::PathBuf;

use config::Config;

use crate::Result;

#[derive(Debug)]
pub struct BookOptions {
    pub project: PathBuf,
}

pub fn list(options: BookOptions) -> Result<()> {
    let mut spaces: Vec<Config> = Vec::new();
    workspace::find(&options.project, true, &mut spaces)?;
    for config in spaces {
        book::list(&config)?;
    }
    Ok(())
}
