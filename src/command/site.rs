use std::path::PathBuf;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use log::{info, warn, error};

use crate::cache;
use crate::config::Config;
use crate::Error;
use crate::Result;
use crate::utils;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SiteManifest {
    pub sites: HashMap<String, SiteManifestEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SiteManifestEntry {
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

fn save(manifest: SiteManifest) -> Result<()> {
    let file = cache::get_workspace_manifest()?;
    let content = toml::to_string(&manifest)?;
    utils::write_string(file, content)?;
    Ok(())
}

pub fn add(options: AddOptions) -> Result<()> {
    let mut manifest = load()?;

    // Must have a valid config
    let config = Config::load(&options.project, false)?;
    let project = config.get_project().canonicalize()?;

    // Use specific name or infer from the directory name
    let mut name = "".to_string();
    if let Some(ref project_name) = options.name {
        name = project_name.to_string()
    } else {
        if let Some(ref file_name) = project.file_name() {
            name = file_name.to_string_lossy().into_owned();
        }
    }

    if name.is_empty() {
        return Err(
            Error::new(format!("Could not determine site name")));
    }

    if manifest.sites.contains_key(&name) {
        return Err(
            Error::new(format!("Site '{}' already exists", name)));
    }

    let mut link_target = cache::get_workspace_dir()?;
    link_target.push(&name);

    if link_target.exists() {
        std::fs::remove_file(&link_target)?;
    }

    utils::symlink::soft(&project, &link_target)?;

    let entry = SiteManifestEntry { project };
    manifest.sites.insert(name.clone(), entry);
    save(manifest)?;

    info!("Added {}", &name);

    Ok(())
}

pub fn remove(options: RemoveOptions) -> Result<()> {
    let mut manifest = load()?;

    if !manifest.sites.contains_key(&options.name) {
        return Err(
            Error::new(format!("Site '{}' does not exist", &options.name)));
    }

    // Remove the symlink
    let mut link_target = cache::get_workspace_dir()?;
    link_target.push(&options.name);
    if let Err(e) = std::fs::remove_file(&link_target) {
        warn!("Unable to remove symlink: {}", e);
    }

    // Update the manifest
    manifest.sites.remove(&options.name);
    save(manifest)?;

    info!("Removed {}", &options.name);

    Ok(())
}

pub fn list(_options: ListOptions) -> Result<()> {
    let manifest = load()?;
    if manifest.sites.is_empty() {
        info!("No sites yet");
    } else {
        for (name, site) in manifest.sites {
            let ok = Config::load(&site.project, false).is_ok();
            if ok {
                info!("{} -> {}", name, site.project.display());
            } else {
                error!("{} -> {} [invalid]", name, site.project.display());
            }
        } 
    }
    Ok(())
}
