use tokio::sync::oneshot;

use log::info;

use crate::Error;
use config::server::{ServerConfig, LaunchConfig, ConnectionInfo};
use super::{router, Channels};

/// Start a server and launch a browser window.
pub async fn launch(
    options: &'static ServerConfig,
    launch: LaunchConfig,
    channels: &mut Channels,
) -> Result<(), Error> {

    // Create a channel to receive the bind address.
    let (ctx, crx) = oneshot::channel::<ConnectionInfo>();
    channels.bind = Some(ctx);

    let _ = tokio::task::spawn(async move {
        let info = crx.await.unwrap();

        let url = info.to_url();
        info!("Serve {}", &url);

        // Most of the time we want to open a browser unless explictly
        // disabled however in the case of the live reload logic it 
        // takes control of opening the browser so that:
        //
        // 1) Don't start to compile until we have bound to a port.
        // 2) Don't open a browser window unless the build succeeds.
        // 
        if launch.open {
            // It is ok if this errors we just don't open a browser window
            open::that(&url).map(|_| ()).unwrap_or(());
        }
    });

    Ok(start(options, channels).await?)
}

/// Start a server.
pub async fn start(
    options: &'static ServerConfig,
    channels: &mut Channels,
) -> Result<(), Error> {
    Ok(router::serve(options, channels).await?)
}
