use std::path::PathBuf;

use crate::{Error, Result};
use config::server::{LaunchConfig, ServerConfig};

pub async fn get_target() -> Result<PathBuf> {
    // Served from a sub-directory
    let target = dirs::docs_dir()?;
    if !target.exists() {
        return Err(Error::NotDirectory(target));
    }

    Ok(target)
}

pub async fn open(opts: ServerConfig) -> Result<()> {
    let launch = LaunchConfig { open: true };

    // Convert to &'static reference
    let opts = server::configure(opts);
    let mut channels = Default::default();
    Ok(server::launch(opts, launch, &mut channels).await?)
}
