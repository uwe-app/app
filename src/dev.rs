use std::path::Path;

use config::ProfileSettings;
use crate::{Error, opts::fatal};

fn server_error_cb(e: server::Error) {
    let _ = fatal(Error::from(e));
}

pub async fn run<P: AsRef<Path>>(
    project: P,
    mut args: ProfileSettings,
) -> Result<(), Error> {
    let project = project.as_ref();
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project.to_path_buf()));
    }

    // Prepare the server settings
    let port = args.get_port().clone();
    if port == 0 {
        return Err(Error::NoLiveEphemeralPort);
    }

    // Must mark the build profile for live reload
    args.live = Some(true);

    // Compile the project
    let result = workspace::compile(project, &args).await?;

    // Start the webserver
    server::watch(
        port,
        args.tls.clone(),
        args.launch.clone(),
        result,
        server_error_cb,
    )
    .await?;

    Ok(())
}
