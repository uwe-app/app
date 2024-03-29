use std::fs::{remove_dir_all, File};
use std::path::PathBuf;

use log::info;
use scopeguard::defer;

use config::plugin::Plugin;

use crate::{
    linter::lint, packager, registry, registry::RegistryAccess, Error, Result,
};

/// Publish a plugin.
pub async fn publish(source: &PathBuf) -> Result<(PathBuf, Vec<u8>, Plugin)> {
    let plugin = lint(source).await?;
    //lint_plugin(&plugin)?;

    let registry_path = option_env!("UPM_PUBLISH")
        .expect("Publish registry path environment variable not set");
    let registry_repo = option_env!("UPM_PUBLISH_REPO")
        .expect("Publish repository environment variable not set");

    // This is a mis-configuration of the environment variable
    let repo_path = PathBuf::from(registry_repo);
    if !repo_path.exists() || !repo_path.is_dir() {
        return Err(Error::NotDirectory(repo_path));
    }

    let repo = scm::open(registry_repo)?;
    if !scm::is_clean(&repo) {
        return Err(Error::RegistryNotClean(registry_repo.to_string()));
    }

    // Pull latest version of the reader registry
    scm::system_repo::fetch_registry().await?;

    let writer = PathBuf::from(registry_path);
    let reader = writer.clone();

    let registry = registry::RegistryFileAccess::new(reader, writer)?;

    let entry = registry.entry(plugin.name()).await?;

    if let Some(ref entry) = entry {
        if let Some(_) = entry.get(plugin.version()) {
            return Err(Error::RegistryPluginVersionExists(plugin.to_string()));
        }
    }

    let dir = tempfile::tempdir()?.into_path();
    let target = dir.join(config::PACKAGE);
    defer! {
        let _ = remove_dir_all(&dir);
    }

    let (pkg, digest, plugin) =
        packager::pack_plugin(source, &target, plugin).await?;

    let pkg_file = File::open(&pkg)?;
    let size = pkg_file.metadata()?.len();
    info!("Archive {} ({} bytes)", pkg.display(), size);
    info!("Checksum {}", hex::encode(&digest));
    upload(&pkg, &plugin).await?;

    // Inject version into the registry and save the changes
    let mut entry = entry.unwrap_or(Default::default());
    let entry_file = registry.register(&mut entry, &plugin, &digest).await?;

    let id = plugin.to_string();

    // Commit the updated registry entry
    let rel = entry_file.strip_prefix(&repo_path)?;
    let msg = format!("Plugin publish {}.", &id);
    scm::commit_file(&repo, &rel, &msg)?;

    info!("Push {}", repo_path.display());
    scm::push_remote_name(&repo, scm::ORIGIN, None, None)?;

    info!("Published {} ✓", &id);

    Ok((pkg, digest, plugin))
}

/// Upload the plugin package to the s3 bucket.
async fn upload(pkg: &PathBuf, plugin: &Plugin) -> Result<()> {
    let registry_profile = option_env!("UPM_PUBLISH_PROFILE")
        .expect("Publish profile environment variable not set");
    let registry_region = option_env!("UPM_PUBLISH_REGION")
        .expect("Publish region environment variable not set");
    let registry_bucket = option_env!("UPM_PUBLISH_BUCKET")
        .expect("Publish bucket environment variable not set");

    let region = publisher::parse_region(registry_region)?;
    let key = format!(
        "{}/{}/{}.tar.xz",
        plugin.name(),
        plugin.version().to_string(),
        config::PACKAGE
    );

    info!("Upload {} ({})", registry_bucket, registry_region);
    publisher::put_object_file_once(
        registry_profile,
        &region,
        registry_bucket,
        &key,
        pkg,
    )
    .await?;
    info!("{} ✓", &key);

    Ok(())
}
