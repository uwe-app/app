use std::path::PathBuf;

use log::info;

use crate::Result;

#[derive(Debug)]
pub struct PluginOptions {
    pub path: PathBuf,
}

pub async fn lint(options: PluginOptions) -> Result<()> {
    let plugin = plugin::read(&options.path).await?;
    plugin::lint(&plugin)?;
    info!("Plugin {} ok ✓", &plugin.name);
    Ok(())
}
