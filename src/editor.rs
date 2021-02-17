use std::env;
use std::path::{Path, PathBuf};

use crate::{error::server_error_cb, Error, Result};
use config::ProfileSettings;
use workspace::{HostNameMode, HostSettings};

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

    let headless = env::var("UWE_HEADLESS").ok().is_some();
    let editor_directory = env::var("UWE_EDITOR").ok().map(PathBuf::from);

    let settings = HostSettings {
        host_name: HostNameMode::Always,
    };

    // Compile the project
    let result = workspace::compile(project, &args, settings).await?;

    // Start the webserver
    server::watch(
        port,
        args.tls.clone(),
        args.launch.clone(),
        headless,
        result,
        true,
        editor_directory,
        args.host.clone(),
        authorities,
        server_error_cb,
    )
    .await?;

    Ok(())
}
