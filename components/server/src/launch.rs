use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;

use log::info;

use crate::{router, Error, channels::Channels};
use config::server::{ConnectionInfo, LaunchConfig, ServerConfig};

/// Start a server and launch a browser window.
pub async fn launch(
    options: &'static ServerConfig,
    launch: LaunchConfig,
    //channels: Arc<RwLock<Channels>>,
) -> Result<(), Error> {

    // Create a channel to receive the bind address.
    let (ctx, crx) = oneshot::channel::<ConnectionInfo>();

    let channels = Channels::new(ctx);

    //channels.bind = Some(ctx);

    //let mut channels_writer = 

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

    Ok(start(options, Arc::new(RwLock::new(channels))).await?)
}

/// Start a server.
pub async fn start(
    options: &'static ServerConfig,
    channels: Arc<RwLock<Channels>>,
) -> Result<(), Error> {
    Ok(router::serve(options, channels).await?)
}
