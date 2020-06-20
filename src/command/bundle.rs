use std::fs;
use std::path::PathBuf;
use crate::Error;

use crate::cache;
use crate::git;
use crate::preference;
use crate::utils;
use crate::bundle::*;

use log::{info, debug};

static ASSETS: &str = "assets.go";

#[derive(Debug)]
pub struct BundleOptions {
    pub source: PathBuf,
    pub target: PathBuf,
    pub force: bool,
    pub name: Option<String>,
    pub keep: bool,
    pub linux: bool,
    pub mac: bool,
    pub windows: bool,
}

fn prepare(options: &BundleOptions) -> Result<Vec<PathBuf>, Error> {
    let mut sources: Vec<PathBuf> = Vec::new();

    if options.target.exists() {
        if options.force {
            info!("rm -rf {}", options.target.display());
            fs::remove_dir_all(&options.target)?;
        } else {
            return Err(
                Error::new(
                    format!(
                        "{} already exists, use --force to overwrite", options.target.display())))
        }
    }

    let prefs = preference::load()?;

    let standalone_dir = cache::get_standalone_dir()?;
    if !standalone_dir.exists() {
        cache::update(&prefs, vec![cache::CacheComponent::Standalone])?;
    }

    let from = standalone_dir.to_string_lossy();

    git::print_clone(&from, &options.target);
    git::clone_standard(&from, &options.target)?;

    let repo = git::open_repo(&options.target)?;

    // Remove the .git directory
    fs::remove_dir_all(repo.path())?;

    for file in options.target.read_dir()? {
        let entry = file?;
        let path = entry.path();
        if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            if !name.ends_with(".go") {
                if path.is_file() {
                    fs::remove_file(path)?;
                }
            } else if name == ASSETS.to_string() {
                fs::remove_file(path)?;
            } else {
                sources.push(path.to_path_buf());
            }
        }
    }

    Ok(sources)
}

pub fn bundle(options: BundleOptions) -> Result<(), Error> {
    let mut sources = prepare(&options)?;

    // Try to work out a default name from the current folder
    let mut name = "".to_string();
    if let Some(nm) = options.name {
        name = nm;
    } else {
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(nm) = cwd.file_name() {
                name = nm.to_string_lossy().into_owned();
            }
        }
    }

    let bundler = Bundler::new();
    if let Err(_) = bundler.version() {
        return Err(
            Error::new(
                "could not execute 'go version', install from https://golang.org/dl/".to_string()))
    }

    info!("bundle {} -> {}", options.source.display(), options.target.display());

    let mut dest = options.target.clone();
    dest.push("assets.go");
    sources.push(dest.clone());

    let content = bundler.generate(&options.source)?;
    utils::write_string(dest, content)?;

    let mut linux = options.linux;
    let mut mac = options.mac;
    let mut windows = options.windows;

    // No flags given so build all target platforms
    if !linux && !mac && !windows {
        linux = true; mac = true; windows = true;
    }

    // Set up default targets
    let mut targets: Vec<Target> = Vec::new();
    if linux {
        targets.push(Target{platform: Platform::linux(), arch: Arch::amd64()});
    }
    if mac {
        targets.push(Target{platform: Platform::darwin(), arch: Arch::amd64()});
    }
    if windows {
        targets.push(Target{platform: Platform::windows(), arch: Arch::amd64()});
    }

    let executables = bundler.compile(&options.target, &name, targets)?;

    if !options.keep {
        for src in sources {
            debug!("rm {}", src.display());
            std::fs::remove_file(src)?;
        }
    }

    for exe in executables {
        info!("{}", exe.display());
    }

    Ok(())
}
