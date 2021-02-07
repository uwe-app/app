use std::path::Path;

use crate::{opts::fatal, Error};
use config::ProfileSettings;

fn server_error_cb(e: server::Error) {
    let _ = fatal(Error::from(e));
}

pub async fn run<P: AsRef<Path>>(
    project: P,
    mut args: ProfileSettings,
) -> Result<(), Error> {
    // Prepare the server settings
    let port = args.get_port().clone();
    if port == 0 {
        return Err(Error::NoLiveEphemeralPort);
    }

    // Must mark the build profile for live reload
    args.live = Some(true);

    let headless = option_env!("UWE_HEADLESS").is_some();

    // Compile the project
    let result = workspace::compile(project, &args).await?;

    // Start the webserver
    server::watch(
        port,
        args.tls.clone(),
        args.launch.clone(),
        headless,
        result,
        false,
        server_error_cb,
    )
    .await?;

    Ok(())
}
