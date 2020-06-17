use std::fs;
use std::path::Path;
use std::path::PathBuf;

use url::Url;
use git2::Repository;
use log::info;

use crate::blueprint;
use crate::Error;

#[derive(Debug)]
pub struct InitOptions {
    pub source: String,
    pub target: Option<PathBuf>,
    pub list: bool,
    pub fetch: bool,
}

fn create<P: AsRef<Path>>(target: P, options: &InitOptions) -> Result<(), Error> {
    println!("{:?}", options);

    let (repo, base, _cloned) = blueprint::open_or_clone()?;
    match Url::parse(&options.source) {
        Ok(_) => {
            info!("Clone {}", &options.source);
            info!("   -> {}", target.as_ref().display());
            Repository::clone(&options.source, target)?;
            return Ok(())
        },
        Err(e) => {
            let modules = repo.submodules()?;
            for sub in modules {
                if sub.path() == Path::new(&options.source) {
                    let mut tmp = base.clone();
                    tmp.push(sub.path());
                    let src = tmp.to_string_lossy();
                    info!("Clone {}", tmp.display());
                    info!("   -> {}", target.as_ref().display());
                    Repository::clone(&src, target)?;
                    return Ok(())
                }
            }

            // Now we have SSH style git@github.com: URLs to deal with
        }
    }

    Err(Error::new(format!("Unable to handle source specification")))
}

// TODO: support [blueprint] default config

pub fn init(options: InitOptions) -> Result<(), Error> {

    if options.list {
        let (repo, _base, _cloned) = blueprint::open_or_clone()?;
        blueprint::list_submodules(repo)?;
    } else if options.fetch {
        blueprint::clone_or_fetch()?;
    } else {

        if let Some(ref target) = options.target {
            if target.exists() {
                return Err(
                    Error::new(
                        format!("Target '{}' exists, please move it away", target.display())));
            }

            if let Some(ref parent) = target.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
                create(target, &options)?
            } else {
                create(target, &options)?
            }
        } else {
            return Err(Error::new(format!("Target directory is required")));
        }
    }

    Ok(())
}
