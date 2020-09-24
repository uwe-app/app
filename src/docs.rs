use std::path::PathBuf;

use crate::Result;
use cache::{self, CacheComponent};
use config::server::{LaunchConfig, ServerConfig};

static DOCS_DIR: &str = "docs";

pub async fn get_target() -> Result<PathBuf> {
    // Served from a sub-directory
    let target = cache::get_docs_dir()?;

    if !target.exists() {
        let prefs = preference::load()?;
        cache::update(&prefs, vec![CacheComponent::Documentation])?;
    }

    Ok(target.join(DOCS_DIR))
}

pub async fn open(opts: ServerConfig) -> Result<()> {
    let launch = LaunchConfig { open: true };

    // Convert to &'static reference
    let opts = server::configure(opts);
    let mut channels = Default::default();
    Ok(server::launch(opts, launch, &mut channels).await?)
}
