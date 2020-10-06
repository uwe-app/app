use std::path::PathBuf;

use human_bytes::human_bytes;
use log::{debug, info};

use crate::{Error, Result};

/// Lint a plugin.
pub async fn lint(path: PathBuf, inspect: bool) -> Result<()> {
    let plugin = plugin::lint(path).await?;
    if inspect {
        println!("{}", toml::to_string(&plugin)?);
    }
    info!("Plugin {} ok ✓", &plugin.name);
    Ok(())
}

/// Package a plugin.
pub async fn pack(path: PathBuf) -> Result<()> {
    let target = path.join(config::PACKAGE);
    let source = path;
    let (pkg, digest, plugin) = plugin::pack(&source, &target).await?;
    let size = pkg.metadata()?.len();
    debug!("{}", hex::encode(digest));
    info!("{}", plugin.to_string());
    info!("{} ({})", pkg.display(), human_bytes(size as f64));
    Ok(())
}

/// Publish a plugin.
pub async fn publish(path: PathBuf) -> Result<()> {
    let registry_path = option_env!("AB_PUBLISH");
    let registry_repo = option_env!("AB_PUBLISH_REPO");

    if registry_path.is_none() || registry_repo.is_none() {
        log::warn!("Plugin publishing is not available yet.");
        log::warn!("");
        log::warn!("During the alpha and beta plugins are curated, ");
        log::warn!("you may still contribute a plugin by adding ");
        log::warn!("a PR to this repository:");
        log::warn!("");
        log::warn!("https://github.com/uwe-app/plugins");
        log::warn!("");

        return Err(Error::NoPluginPublishPermission);
    }

    plugin::publish(&path).await?;

    Ok(())
}
