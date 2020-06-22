use std::path::Path;
use std::path::PathBuf;

use crate::Error;
use crate::cache::{self, CacheComponent};
use crate::preference;
use crate::utils::symlink;

static NAME: &str = "ht";

#[derive(Default)]
pub struct VersionInfo {
    version: String,
}

pub fn version() -> Result<VersionInfo, Error> {
    Ok(Default::default())
}

pub fn update() -> Result<PathBuf, Error>  {
    let prefs = preference::load()?;
    let components = vec![CacheComponent::Release];
    cache::update(&prefs, components)?;

    let mut bin = cache::get_bin_dir()?;
    let mut release_bin = cache::get_release_bin_dir()?;

    bin.push(NAME);
    release_bin.push(NAME);

    if bin.exists() {
        std::fs::remove_file(&bin)?;
    }

    if !bin.exists() {
        symlink::soft(&release_bin, &bin)?; 
    }

    Ok(bin)
}
