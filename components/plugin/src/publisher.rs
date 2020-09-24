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
    let registry_repo = option_env!("PUBLISH_AB_REPO").unwrap();

    // Pull latest version of the reader registry
    let prefs = preference::load()?;
    cache::update(&prefs, vec![cache::CacheComponent::Runtime])?;

    let reader = cache::get_registry_dir()?;
    let writer = PathBuf::from(registry_path);

    let repo = PathBuf::from(registry_repo);

    // This is a mis-configuration of the environment variable
    if !repo.exists() || !repo.is_dir() {
        return Err(Error::NotDirectory(repo));
    }

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
