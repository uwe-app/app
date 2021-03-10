use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{RwLock};

use once_cell::sync::OnceCell;

use serde::{Deserialize, Serialize};

use config::Config;

use crate::{Error, Result};

pub type ProjectList = HashSet<ProjectStatus>;

fn manifest() -> &'static RwLock<ProjectManifest> {
    static INSTANCE: OnceCell<RwLock<ProjectManifest>> = OnceCell::new();
    INSTANCE.get_or_init(|| RwLock::new(ProjectManifest {project: HashSet::new()}))
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum SettingsStatus {
    /// Settings file does not exist
    #[serde(rename = "missing")]
    Missing,
    /// Settings file could not be parsed
    #[serde(rename = "error")]
    Error,
    /// Settings files is ok
    #[serde(rename = "ok")]
    Ok,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectManifest {
    pub project: HashSet<ProjectManifestEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct ProjectManifestEntry {
    pub id: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ProjectStatus {
    pub status: SettingsStatus,
    pub entry: ProjectManifestEntry,
}

/// Load from the disc to the in-memory store.
pub fn load() -> Result<()> {
    let file = dirs::projects_manifest()?;
    if !file.exists() {
        return Ok(());
    }
    let contents = utils::fs::read_string(file)?;
    let mut backing: ProjectManifest = toml::from_str(&contents)?;

    // Set up opaque identifiers from project paths
    let project = backing
        .project
        .drain()
        .map(|mut e| {
            e.id = Some(crate::checksum(&e.path).unwrap());
            e
        })
        .collect::<HashSet<_>>();

    let mut manifest = manifest().write().unwrap();
    *manifest = ProjectManifest { project };

    Ok(())
}

// Save a copy of the in-memory project manifest back to disc.
fn flush(manifest: ProjectManifest) -> Result<()> {
    let file = dirs::projects_manifest()?;
    let content = toml::to_string(&manifest)?;
    utils::fs::write_string(file, content)?;
    Ok(())
}

/// Add a project to the manifest.
///
/// The project is added to the in-memory store and flushed to disc.
pub fn add(mut entry: ProjectManifestEntry) -> Result<()> {
    let mut manifest = manifest().write().unwrap();

    if entry.path.is_relative() {
        return Err(Error::NoRelativeProject(entry.path.to_path_buf()))
    }

    // Must have a valid config
    let _ = Config::load(&entry.path, false)?;

    let existing = manifest.project
        .iter().find(|p| &p.path == &entry.path);
    if existing.is_some() {
        return Err(Error::Exists(entry.path.to_path_buf()));
    }

    if entry.id.is_none() {
        entry.id = Some(crate::checksum(&entry.path)?);
    }

    manifest.project.insert(entry);
    flush(manifest.clone())?;

    Ok(())
}

/// Remove a project from the manifest.
///
/// The project is removed from the in-memory store and flushed to disc.
pub fn remove(entry: &ProjectManifestEntry) -> Result<()> {
    let mut manifest = manifest().write().unwrap();

    // Update the manifest
    let removed = manifest.project.remove(&entry);

    if removed {
        flush(manifest.clone())?;
        Ok(())
    } else {
        Err(Error::NotExists(entry.path.to_path_buf()))
    }
}

/// List projects and check if the project settings can be loaded.
pub fn list() -> Result<HashSet<ProjectStatus>> {
    let mut projects: HashSet<ProjectStatus> = HashSet::new();

    let manifest = manifest().read().unwrap();
    for entry in manifest.project.iter() {
        let settings_file = entry.path.join(config::SITE_TOML);
        let status = if settings_file.exists() {
            match Config::load(&entry.path, false) {
                Ok(_) => SettingsStatus::Ok,
                Err(_) => SettingsStatus::Error,
            }
        } else {
            SettingsStatus::Missing
        };
        let item = ProjectStatus { status, entry: entry.clone() };
        projects.insert(item);
    }
    Ok(projects)
}
