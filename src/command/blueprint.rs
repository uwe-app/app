use std::fs;
use std::path::Path;
use std::path::PathBuf;

use git2::Repository;
use home;
use log::info;
use url::Url;

use crate::preference::{self, Preferences};
use crate::{cache, git, Error};

#[derive(Debug)]
pub struct InitOptions {
    pub source: Option<String>,
    pub target: Option<PathBuf>,
    pub private_key: Option<PathBuf>,
}

fn create<P: AsRef<Path>>(
    target: P,
    options: &InitOptions,
    prefs: &Preferences,
) -> Result<Repository, Error> {
    let mut src = "".to_string();

    if let Some(ref source) = options.source {
        src = source.clone();
    } else {
        if let Some(ref source) = prefs.blueprint.as_ref().unwrap().default_path {
            src = source.clone();
        }
    }

    if src.is_empty() {
        return Err(Error::new(format!(
            "Could not determine default source path"
        )));
    }

    let src_err = Err(Error::new(format!("Unable to handle source '{}'", &src)));

    let repo_url = cache::get_blueprint_url(prefs);
    let repo_dir = cache::get_blueprint_dir()?;
    let (repo, _cloned) = git::open_or_clone(&repo_url, &repo_dir, true)?;
    match Url::parse(&src) {
        Ok(_) => {
            git::print_clone(&src, target.as_ref().clone());
            return Repository::clone(&src, target).map_err(Error::from);
        }
        Err(_) => {
            let modules = repo.submodules()?;
            for sub in modules {
                if sub.path() == Path::new(&src) {
                    let mut tmp = repo_dir.clone();
                    tmp.push(sub.path());
                    let src = tmp.to_string_lossy().into_owned();
                    git::print_clone(&src, target.as_ref().clone());
                    return Repository::clone(&src, target).map_err(Error::from);
                }
            }

            let ssh_req = src.trim_start_matches("ssh://");
            let ssh_url = format!("ssh://{}", &ssh_req);

            match Url::parse(&ssh_url) {
                Ok(url) => {
                    if url.username().is_empty() {
                        log::warn!("No username for source URL");
                        log::warn!("Perhaps you want a blueprint submodule, try `ht init --list`");
                        return src_err;
                    }

                    // Now we have SSH style git@github.com: URLs to deal with
                    if let Some(mut key_file) = home::home_dir() {
                        if let Some(ref ssh_key) = options.private_key {
                            key_file.push(ssh_key);

                            info!("Private key {}", key_file.display());

                            git::print_clone(&src, target.as_ref().clone());

                            return git::clone_ssh(src, target, key_file, None);
                        } else {
                            return Err(Error::new(format!(
                                "To use SSH specify the --ssh-key option"
                            )));
                        }
                    }
                }
                Err(_) => return src_err,
            }
        }
    }

    src_err
}

pub fn list() -> Result<(), Error> {
    let prefs = preference::load()?;

    let url = cache::get_blueprint_url(&prefs);
    let blueprint_cache_dir = cache::get_blueprint_dir()?;

    if !blueprint_cache_dir.exists() {
        git::print_clone(&url, &blueprint_cache_dir);
    }

    let (repo, _cloned) = git::open_or_clone(&url, &blueprint_cache_dir, true)?;
    git::list_submodules(repo)?;

    Ok(())
}

pub fn init(options: InitOptions) -> Result<(), Error> {
    let prefs = preference::load()?;

    let url = cache::get_blueprint_url(&prefs);
    let blueprint_cache_dir = cache::get_blueprint_dir()?;

    if !blueprint_cache_dir.exists() {
        git::print_clone(&url, &blueprint_cache_dir);
    }

    //if options.list {
        //let (repo, _cloned) = git::open_or_clone(&url, &blueprint_cache_dir, true)?;
        //git::list_submodules(repo)?;
    //} else {
        if let Some(ref target) = options.target {
            if target.exists() {
                return Err(Error::new(format!(
                    "Target '{}' exists, please move it away",
                    target.display()
                )));
            }

            let repo;
            if let Some(ref parent) = target.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
                repo = create(target, &options, &prefs)?;
            } else {
                repo = create(target, &options, &prefs)?;
            }

            //repo.remote_delete("origin")?;

            // FIXME: support tracking upstream blueprint
            git::detached(target, repo)?;
        } else {
            return Err(Error::new(format!("Target directory is required")));
        }
    //}

    Ok(())
}
