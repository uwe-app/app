use crate::Result;
use config::server::{LaunchConfig, ServerConfig};

pub async fn serve(opts: ServerConfig, launch: LaunchConfig) -> Result<()> {
    // Convert to &'static reference
    let opts = server::configure(opts);
    let mut channels = Default::default();
    Ok(server::launch(opts, launch, &mut channels).await?)
}
