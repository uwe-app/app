use std::fs;
use std::path::PathBuf;
use crate::Error;

use crate::utils;
use crate::bundle::Bundler;

use log::info;


#[derive(Debug)]
pub struct BundleOptions {
    pub source: PathBuf,
    pub target: PathBuf,
    pub force: bool,
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

    for f in copy_files.iter() {
        utils::copy_asset_bundle_file(f, base_name, &target)?;
    }

    let content = bundler.generate(&options.source)?;
    let mut dest = target.clone();
    dest.push("assets.go");
    //println!("{}", content);
    //println!("{}", dest.display());

    utils::write_string(dest, content)?;

    bundler.compile(&target)?;

    Ok(())
}
