use std::path::Path;

use config::{lock_file::LockFile, ProfileSettings};

use scopeguard::defer;
use workspace::lock;

use crate::{Error, opts::fatal};

pub async fn compile<P: AsRef<Path>>(
    project: P,
    args: ProfileSettings,
) -> Result<(), Error> {
    let project = project.as_ref();
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project.to_path_buf()));
    }

    let lock_path = LockFile::get_lock_file(project);
    let lock_file = lock::acquire(&lock_path)?;
    defer! { let _ = lock::release(lock_file); }

    let is_live = args.live.is_some() && args.live.unwrap();
    if is_live {
        live(project, args).await?;
    } else {
        workspace::compile(project, &args).await?;
    }

    Ok(())
}

fn server_error_cb(e: server::Error) {
    let _ = fatal(Error::from(e));
}

async fn live<P: AsRef<Path>>(
    project: P,
    args: ProfileSettings,
) -> Result<(), Error> {
    // Prepare the server settings
    let port = args.get_port().clone();
    if port == 0 {
        return Err(Error::NoLiveEphemeralPort);
    }

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
