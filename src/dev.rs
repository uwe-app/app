use std::path::Path;

use crate::{error::server_error_cb, Error, Result};
use config::ProfileSettings;

pub async fn run<P: AsRef<Path>>(
    project: P,
    mut args: ProfileSettings,
    authorities: Option<Vec<String>>,
) -> Result<()> {
    // Prepare the server settings
    let port = args.get_port().clone();
    if port == 0 {
        return Err(Error::NoLiveEphemeralPort);
    }

    // Must mark the build profile for live reload
    args.live = Some(true);

    let headless = std::env::var("UWE_HEADLESS").ok().is_some();

    // Compile the project
    let result = workspace::compile(project, &args, Default::default()).await?;

    // Start the webserver
    server::watch(
        args.host.clone(),
        port,
        args.tls.clone(),
        args.launch.clone(),
        headless,
        result,
        authorities,
        server_error_cb,
    )
    .await?;

    Ok(())
}
