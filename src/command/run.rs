use crate::Result;
use config::server::{ServerConfig, LaunchConfig};

pub async fn serve(opts: ServerConfig, launch: LaunchConfig) -> Result<()> {
    // Convert to &'static reference
    let opts = server::configure(opts);

    Ok(server::bind(opts, launch, None, None).await?)
}
