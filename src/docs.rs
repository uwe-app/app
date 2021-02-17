use config::server::{LaunchConfig, ServerConfig};

use crate::Result;

pub async fn open(opts: ServerConfig) -> Result<()> {
    let launch = LaunchConfig { open: true };

    // Convert to &'static reference
    let opts = server_actix::configure(opts);
    Ok(server_actix::launch(opts, launch).await?)
}
