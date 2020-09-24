use std::path::PathBuf;

use log::{info, debug};
use human_bytes::human_bytes;

use crate::{Error, Result};

#[derive(Debug)]
pub struct PluginOptions {
    pub path: PathBuf,
}

/// Lint a plugin.
pub async fn lint(options: PluginOptions) -> Result<()> {
    let plugin = plugin::read(&options.path).await?;
    plugin::lint(&plugin)?;
    info!("Plugin {} ok ✓", &plugin.name);
    Ok(())
}

/// Package a plugin.
pub async fn pack(options: PluginOptions) -> Result<()> {
    let target = options.path.join(config::PACKAGE);
    let source = options.path;
    let (pkg, digest, plugin) = plugin::pack(&source, &target).await?;
    let size = pkg.metadata()?.len();
    debug!("{}", hex::encode(digest));
    info!("{}", plugin.to_string());
    info!("{} ({})", pkg.display(), human_bytes(size as f64));
    Ok(())
}

/// Publish a plugin.
pub async fn publish(options: PluginOptions) -> Result<()> {
    let registry_path = option_env!("AB_PUBLISH");
    let registry_repo = option_env!("AB_PUBLISH_REPO");

    if registry_path.is_none() || registry_repo.is_none() {
        log::warn!("Plugin publishing is not available yet.");
        log::warn!("");
        log::warn!("During the alpha and beta plugins are curated, ");
        log::warn!("you may still contribute a plugin by adding ");
        log::warn!("a PR to this repository:");
        log::warn!("");
        log::warn!("https://github.com/hypertext-live/plugins");
        log::warn!("");

        return Err(Error::NoPluginPublishPermission)
    }

    plugin::publish(&options.path).await?;

    Ok(())
}
