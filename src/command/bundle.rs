use std::fs;
use std::path::PathBuf;
use crate::Error;

use crate::utils;
use crate::bundle::*;

use log::{info, debug};

#[derive(Debug)]
pub struct BundleOptions {
    pub source: PathBuf,
    pub target: PathBuf,
    pub force: bool,
    pub name: Option<String>,
    pub keep: bool,
}

pub fn bundle(options: BundleOptions) -> Result<(), Error> {

    // The base_name is the folder in the asset bundle
    // and target_dir is the folder we use for generated
    // assets whilst the values are the same the purposes
    // are very different.
    let base_name = "bundle";
    let target_dir = "bundle";

    let copy_files : Vec<&str> = vec![
        "main.go",
        "fs.go",
        "open_darwin.go",
        "open_linux.go",
        "open_windows.go",
    ];

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

    let mut target = options.target.clone();
    target.push(target_dir);

    if target.exists() {
        if options.force {
            info!("rm -rf {}", target.display());
            fs::remove_dir_all(&target)?;
        } else {
            return Err(
                Error::new(
                    format!(
                        "{} already exists, use --force to overwrite", target.display())))
        }
    }

    info!("bundle {} -> {}", options.source.display(), target.display());

    let mut sources: Vec<PathBuf> = Vec::new();

    for f in copy_files.iter() {
        let pth = utils::copy_asset_bundle_file(f, base_name, &target)?;
        sources.push(pth);
    }

    let mut dest = target.clone();
    dest.push("assets.go");
    sources.push(dest.clone());

    let content = bundler.generate(&options.source)?;
    utils::write_string(dest, content)?;

    // Set up default targets
    let mut targets: Vec<Target> = Vec::new();
    targets.push(Target{platform: Platform::linux(), arch: Arch::amd64()});
    targets.push(Target{platform: Platform::darwin(), arch: Arch::amd64()});
    targets.push(Target{platform: Platform::windows(), arch: Arch::amd64()});

    bundler.compile(&target, &name, targets)?;

    if !options.keep {
        for src in sources {
            debug!("rm {}", src.display());
            std::fs::remove_file(src)?;
        }
    }

    Ok(())
}
