use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use log::info;

use crate::cache;
use crate::Result;
use crate::utils;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SiteManifest {
    pub sites: Vec<SiteManifestEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SiteManifestEntry {
    pub name: String,
    pub project: PathBuf,
}

#[derive(Debug)]
pub struct AddOptions {
    pub name: Option<String>,
    pub project: PathBuf,
}

#[derive(Debug)]
pub struct RemoveOptions {
    pub name: String,
}

#[derive(Debug)]
pub struct ListOptions {}

fn load() -> Result<SiteManifest> {
    let file = cache::get_workspace_manifest()?;
    if !file.exists() {
        return Ok(Default::default());
    }
    let contents = utils::read_string(file)?;
    let manifest: SiteManifest = toml::from_str(&contents)?;
    Ok(manifest)
}

pub fn add(options: AddOptions) -> Result<()> {
    println!("Add site: {:?}", options);
    Ok(())
}

pub fn remove(options: RemoveOptions) -> Result<()> {
    println!("Remove site: {:?}", options);
    Ok(())
}

pub fn list(_options: ListOptions) -> Result<()> {
    let manifest = load()?;
    if manifest.sites.is_empty() {
        info!("No sites yet");
    } else {
        for site in manifest.sites {
            info!("Site {} -> {}", site.name, site.project.display());
        } 
    }
    Ok(())
}
