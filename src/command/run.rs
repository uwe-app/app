use crate::Result;
use config::server::{ServerConfig, LaunchConfig};

pub async fn serve(options: ServerConfig, launch: LaunchConfig) -> Result<()> {
    Ok(server::bind(options, launch, None, None).await?)
}
