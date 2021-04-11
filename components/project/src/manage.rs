use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use once_cell::sync::OnceCell;

use serde::{Deserialize, Serialize};

use config::Config;

use crate::{Error, Result};

pub type ProjectList = Vec<ProjectStatus>;

fn manifest() -> &'static RwLock<ProjectManifest> {
    static INSTANCE: OnceCell<RwLock<ProjectManifest>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        RwLock::new(ProjectManifest {
            project: HashSet::new(),
        })
    })
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
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

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Ord, PartialOrd)]
pub struct ProjectManifestEntry {
    pub id: Option<String>,
    pub path: PathBuf,
    pub name: Option<String>,
    pub host: Option<String>,
}

impl Hash for ProjectManifestEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl PartialEq for ProjectManifestEntry {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

#[derive(
    Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
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
pub fn add(mut entry: ProjectManifestEntry) -> Result<String> {
    let mut manifest = manifest().write().unwrap();

    if entry.path.is_relative() {
        return Err(Error::NoRelativeProject(entry.path.to_path_buf()));
    }

    // Must have a valid config
    let _ = Config::load(&entry.path, false)?;

    let existing = manifest.project.iter().find(|p| &p.path == &entry.path);
    if existing.is_some() {
        return Err(Error::Exists(entry.path.to_path_buf()));
    }

    if entry.id.is_none() {
        entry.id = Some(crate::checksum(&entry.path)?);
    }

    let id = entry.id.clone().unwrap();

    manifest.project.insert(entry);
    flush(manifest.clone())?;

    Ok(id)
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

pub fn find(id: &str) -> Result<Option<ProjectManifestEntry>> {
    let manifest = manifest().read().unwrap();
    for entry in manifest.project.iter() {
        let item_id = if let Some(ref id) = entry.id {
            id.to_string()
        } else {
            crate::checksum(&entry.path)?
        };

        if id == &item_id {
            return Ok(Some(entry.clone()));
        }
    }
    Ok(None)
}

/// List projects and check if the project settings can be loaded.
pub fn list() -> Result<ProjectList> {
    let manifest = manifest().read().unwrap();
    let mut projects: Vec<ProjectStatus> =
        Vec::with_capacity(manifest.project.len());

    for entry in manifest.project.iter() {
        let (status, settings) = if !entry.path.exists() || !entry.path.is_dir()
        {
            (SettingsStatus::Missing, None)
        } else {
            match Config::load(&entry.path, false) {
                Ok(settings) => (SettingsStatus::Ok, Some(settings)),
                Err(_) => (SettingsStatus::Error, None),
            }
        };
        let mut entry = entry.clone();
        if entry.name.is_none() {
            entry.name = entry
                .path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned());
        }
        if let Some(settings) = settings {
            entry.host = Some(settings.host().to_string());
        }
        let item = ProjectStatus { status, entry };
        projects.push(item);
    }

    projects.sort();

    Ok(projects)
}

// Import a project path if it does not exist.
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    let manifest = manifest().read().unwrap();
    let path = path.as_ref();
    for item in manifest.project.iter() {
        if item.path == path {
            return true;
        }
    }
    false
}

// Import a project path if it does not exist.
pub fn import<P: AsRef<Path>>(path: P) -> Result<()> {
    let exists = exists(path.as_ref());
    if !exists {
        let name = path
            .as_ref()
            .file_name()
            .map(|s| s.to_string_lossy().into_owned());
        let entry = ProjectManifestEntry {
            id: Some(crate::checksum(path.as_ref()).unwrap()),
            path: path.as_ref().to_path_buf(),
            name,
            host: None,
        };
        add(entry)?;
    }
    Ok(())
}
