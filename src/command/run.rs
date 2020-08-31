use crate::Result;
use config::server::{ServerConfig, LaunchConfig};

pub async fn serve(opts: ServerConfig, launch: LaunchConfig) -> Result<()> {
    // Convert to &'static reference
    let opts = server::configure(opts);
    let channels = Default::default();
    Ok(server::bind(opts, launch, None, &channels).await?)
}
