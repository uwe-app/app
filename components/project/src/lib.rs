use std::collections::HashSet;
use std::path::PathBuf;

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

fn load() -> Result<ProjectManifest> {
    let file = dirs::projects_manifest()?;
    if !file.exists() {
        return Ok(ProjectManifest { projects: HashSet::new() });
    }
    let contents = utils::fs::read_string(file)?;
    let manifest: ProjectManifest = toml::from_str(&contents)?;
    Ok(manifest)
}

fn save(manifest: ProjectManifest) -> Result<()> {
    let file = dirs::projects_manifest()?;
    let content = toml::to_string(&manifest)?;
    utils::fs::write_string(file, content)?;
    Ok(())
}

pub fn add(project: PathBuf, site_name: Option<String>) -> Result<String> {
    let mut manifest = load()?;

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
    save(manifest)?;

    Ok(name)
}

pub fn remove(entry: &ProjectManifestEntry) -> Result<()> {
    //let target = target.as_ref();
    let mut manifest = load()?;

    // Update the manifest
    let removed = manifest.projects.remove(&entry);

    if removed {
        save(manifest)?;
        Ok(())
    } else {
        Err(Error::NotExists(entry.project.to_path_buf()))
    }
}

pub fn list() -> Result<HashSet<ProjectStatus>> {
    let mut projects: HashSet<ProjectStatus> = HashSet::new();
    let manifest = load()?;
    for entry in manifest.projects {
        let ok = Config::load(&entry.project, false).is_ok();
        let status = ProjectStatus { ok, entry };
        projects.insert(status);
    }
    Ok(projects)
}
