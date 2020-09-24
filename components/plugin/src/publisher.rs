use std::path::PathBuf;
use std::fs::remove_dir_all;

use scopeguard::defer;

use config::Plugin;

use crate::{Result, packager, read, lint};

/// Publish a plugin.
pub async fn publish(source: &PathBuf) -> Result<(PathBuf, Vec<u8>, Plugin)> {
    let plugin = read(source).await?;
    lint(&plugin)?;

    // TODO: pull latest version of the registry
    // TODO: check version is not already published (registry)
    // TODO: inject version into the registry and save the changes

    let dir = tempfile::tempdir()?.into_path();
    let target = dir.join(config::PACKAGE);
    defer! {
        let _ = remove_dir_all(&dir);
    }

    println!("Create archive for publish in {}", target.display());

    let (pkg, digest, plugin) = packager::pack_plugin(source, &target, plugin).await?;

    // TODO: upload the archive
    println!("Upload the archve to s3... {} {}", pkg.display(), pkg.metadata()?.len());

    Ok((pkg, digest, plugin))
}
