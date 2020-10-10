use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use config::Config;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not determine site name")]
    EmptyName,

    #[error("Site {name} already exists")]
    Exists { name: String },

    #[error("Site {name} does not exist")]
    NotExists { name: String },

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

#[derive(Debug, Default)]
pub struct SiteStatus {
    pub ok: bool,
    pub entry: SiteManifestEntry,
}

fn load() -> Result<SiteManifest> {
    let file = dirs::sites_manifest()?;
    if !file.exists() {
        return Ok(Default::default());
    }
    let contents = utils::fs::read_string(file)?;
    let manifest: SiteManifest = toml::from_str(&contents)?;
    Ok(manifest)
}

fn save(manifest: SiteManifest) -> Result<()> {
    let file = dirs::sites_manifest()?;
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

    if manifest.sites.contains_key(&name) {
        return Err(Error::Exists { name });
    }

    let mut link_target = dirs::sites_dir()?;
    link_target.push(&name);

    if link_target.exists() {
        std::fs::remove_file(&link_target)?;
    }

    utils::symlink::soft(&project, &link_target)?;

    let entry = SiteManifestEntry { project };
    manifest.sites.insert(name.clone(), entry);
    save(manifest)?;

    Ok(name)
}

pub fn remove<S: AsRef<str>>(target: S) -> Result<()> {
    let name = target.as_ref();
    let mut manifest = load()?;

    if !manifest.sites.contains_key(name) {
        return Err(Error::NotExists {
            name: name.to_string(),
        });
    }

    // Remove the symlink
    let mut link_target = dirs::sites_dir()?;
    link_target.push(&name);
    std::fs::remove_file(&link_target)?;

    // Update the manifest
    manifest.sites.remove(name);
    save(manifest)?;

    Ok(())
}

pub fn list() -> Result<HashMap<String, SiteStatus>> {
    let mut sites: HashMap<String, SiteStatus> = HashMap::new();
    let manifest = load()?;
    for (name, entry) in manifest.sites {
        let ok = Config::load(&entry.project, false).is_ok();
        let status = SiteStatus { ok, entry };
        sites.insert(name.to_string(), status);
    }
    Ok(sites)
}
