use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{RwLock};

use once_cell::sync::OnceCell;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use config::Config;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not determine project name")]
    EmptyName,

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectManifest {
    pub projects: HashSet<ProjectManifestEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct ProjectManifestEntry {
    pub project: PathBuf,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ProjectStatus {
    pub ok: bool,
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
pub fn add(project: PathBuf, site_name: Option<String>) -> Result<String> {
    let mut manifest = manifest().write().unwrap();

    // Must have a valid config
    let config = Config::load(&project, false)?;
    let project = config.project().canonicalize()?;

    // Use specific name or infer from the directory name
    let mut name = "".to_string();
    if let Some(ref project_name) = site_name {
        name = project_name.to_string()
    } else {
        if let Some(ref file_name) = project.file_name() {
            name = file_name.to_string_lossy().into_owned();
        }
    }

    if name.is_empty() {
        return Err(Error::EmptyName);
    }

    let existing = manifest.projects
        .iter().find(|p| &p.project == &project);
    if existing.is_some() {
        return Err(Error::Exists(project.to_path_buf()));
    }

    let entry = ProjectManifestEntry { project };
    manifest.projects.insert(entry);
    flush(manifest.clone())?;

    Ok(name)
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
        let ok = Config::load(&entry.project, false).is_ok();
        let status = ProjectStatus { ok, entry: entry.clone() };
        projects.insert(status);
    }
    Ok(projects)
}
