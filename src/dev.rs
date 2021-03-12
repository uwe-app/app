use std::path::Path;

use crate::{error::server_error_cb, Result};
use config::ProfileSettings;

pub async fn run<P: AsRef<Path>>(
    project: P,
    headless: bool,
    mut args: ProfileSettings,
    authorities: Option<Vec<String>>,
) -> Result<()> {
    // Prepare the server settings
    let port = args.get_port().clone();

    // Must mark the build profile for live reload
    args.live = Some(true);

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
