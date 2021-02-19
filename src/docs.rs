use config::server::{LaunchConfig, ServerConfig};

use crate::Result;

pub async fn open(opts: ServerConfig) -> Result<()> {
    let launch = LaunchConfig { open: true };

    // Convert to &'static reference
    let opts = server::configure(opts);
    Ok(server::launch(opts, launch).await?)
}
