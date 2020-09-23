use std::path::PathBuf;

use log::info;

use futures::TryFutureExt;

use crate::Result;

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
    let plugin = plugin::read(&options.path).await?;
    plugin::lint(&plugin)?;

    let writer = plugin::PackageWriter::new(options.path)
        .destination("package", true)?
        .tar()
        .and_then(|b| b.xz())
        .await?;

    println!("Package the plugin...");
    Ok(())
}
