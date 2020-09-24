use std::path::PathBuf;
use std::fs::remove_dir_all;

use scopeguard::defer;

use config::{plugin::Plugin};

use crate::{Error, Result, packager, read, lint, registry, registry::RegistryAccess};

/// Publish a plugin.
pub async fn publish(source: &PathBuf) -> Result<(PathBuf, Vec<u8>, Plugin)> {
    let plugin = read(source).await?;
    lint(&plugin)?;

    let registry_path = option_env!("PUBLISH_AB").unwrap();

    // TODO: use cache component repo for the reader side!
    // TODO: pull latest version of the reader registry

    let reader = PathBuf::from(registry_path);
    let writer = PathBuf::from(registry_path);

    // TODO: update the reader side of the registry

    let registry = registry::RegistryFileAccess::new(reader, writer)?;

    let entry = registry.entry(&plugin.name).await?;

    if let Some(ref entry) = entry {
        if let Some(_) = entry.get(&plugin.version) {
            return Err(
                Error::RegistryPluginVersionExists(plugin.to_string()))
        }
    }

    let dir = tempfile::tempdir()?.into_path();
    let target = dir.join(config::PACKAGE);
    defer! {
        let _ = remove_dir_all(&dir);
    }

    println!("Create archive for publish in {}", target.display());

    let (pkg, digest, plugin) = packager::pack_plugin(source, &target, plugin).await?;

    // TODO: upload the archive
    println!("Upload the archve to s3... {} {}", pkg.display(), pkg.metadata()?.len());

    // Inject version into the registry and save the changes
    let mut entry = entry.unwrap_or(Default::default());
    registry.register(&mut entry, &plugin, &digest).await?;

    // TODO: commit and push the repository changes

    Ok((pkg, digest, plugin))
}
