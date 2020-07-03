use std::fs;
use std::path::PathBuf;

use cache;
use git;
use preference::{self, Preferences};

use crate::Error;

#[derive(Debug)]
pub struct InitOptions {
    pub source: Option<String>,
    pub target: Option<PathBuf>,
    pub private_key: Option<PathBuf>,

    pub language: Option<String>,
    pub host: Option<String>,
}

fn prepare() -> Result<(Preferences, String, PathBuf), Error> {
    let prefs = preference::load()?;
    let url = cache::get_blueprint_url(&prefs);
    let blueprint_cache_dir = cache::get_blueprint_dir()?;
    if !blueprint_cache_dir.exists() {
        git::print_clone(&url, &blueprint_cache_dir);
    }
    Ok((prefs, url, blueprint_cache_dir))
}

pub fn list() -> Result<(), Error> {
    let (_, url, blueprint_cache_dir) = prepare()?;
    let (repo, _cloned) = git::open_or_clone(&url, &blueprint_cache_dir, true)?;
    git::list_submodules(repo)?;
    Ok(())
}

pub fn init(options: InitOptions) -> Result<(), Error> {
    let (prefs, _url, _blueprint_cache_dir) = prepare()?;

    if let Some(ref target) = options.target {
        if target.exists() {
            return Err(Error::new(format!(
                "Target '{}' exists, please move it away",
                target.display()
            )));
        }

        let repo;
        let repo_url = cache::get_blueprint_url(&prefs);
        let repo_dir = cache::get_blueprint_dir()?;

        let source = if let Some(ref source) = options.source {
            source.clone()
        } else {
            if let Some(ref source) = prefs.blueprint.as_ref().unwrap().default_path {
                source.clone()
            } else {
                "".to_string()
            }
        };

        if source.is_empty() {
            return Err(Error::new(format!(
                "Could not determine default source path"
            )));
        }

        if let Some(ref parent) = target.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
            repo = git::create(
                source,
                target,
                options.private_key.clone(),
                repo_url,
                repo_dir,
            )?;
        } else {
            repo = git::create(
                source,
                target,
                options.private_key.clone(),
                repo_url,
                repo_dir,
            )?;
        }

        //repo.remote_delete("origin")?;

        // FIXME: support tracking upstream blueprint
        git::detached(target, repo)?;
    } else {
        return Err(Error::new(format!("Target directory is required")));
    }

    Ok(())
}
