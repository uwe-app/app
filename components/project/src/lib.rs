use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{RwLock};

use once_cell::sync::OnceCell;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use config::Config;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Project path {0} is relative, must be an absolute path")]
    NoRelativeProject(PathBuf),

    #[error("Project {0} already exists")]
    Exists(PathBuf),

    #[error("Project {0} does not exist")]
    NotExists(PathBuf),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    Config(#[from] config::Error),
}

type Result<T> = std::result::Result<T, Error>;

fn manifest() -> &'static RwLock<ProjectManifest> {
    static INSTANCE: OnceCell<RwLock<ProjectManifest>> = OnceCell::new();
    INSTANCE.get_or_init(|| RwLock::new(ProjectManifest {projects: HashSet::new()}))
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
    pub projects: HashSet<ProjectManifestEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct ProjectManifestEntry {
    pub project: PathBuf,
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
    let backing: ProjectManifest = toml::from_str(&contents)?;

    let mut manifest = manifest().write().unwrap();
    *manifest = backing;

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
pub fn add(entry: ProjectManifestEntry) -> Result<()> {
    let mut manifest = manifest().write().unwrap();

    if entry.project.is_relative() {
        return Err(Error::NoRelativeProject(entry.project.to_path_buf()))
    }

    // Must have a valid config
    let _ = Config::load(&entry.project, false)?;

    let existing = manifest.projects
        .iter().find(|p| &p.project == &entry.project);
    if existing.is_some() {
        return Err(Error::Exists(entry.project.to_path_buf()));
    }

    manifest.projects.insert(entry);
    flush(manifest.clone())?;

    Ok(())
}

/// Remove a project from the manifest.
///
/// The project is removed from the in-memory store and flushed to disc.
pub fn remove(entry: &ProjectManifestEntry) -> Result<()> {
    let mut manifest = manifest().write().unwrap();

    // Update the manifest
    let removed = manifest.projects.remove(&entry);

    if removed {
        flush(manifest.clone())?;
        Ok(())
    } else {
        Err(Error::NotExists(entry.project.to_path_buf()))
    }
}

/// List projects and check if the project settings can be loaded.
pub fn list() -> Result<HashSet<ProjectStatus>> {
    let mut projects: HashSet<ProjectStatus> = HashSet::new();
    let manifest = manifest().read().unwrap();
    for entry in manifest.projects.iter() {
        let settings_file = entry.project.join(config::SITE_TOML);
        let status = if settings_file.exists() {
            match Config::load(&entry.project, false) {
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
