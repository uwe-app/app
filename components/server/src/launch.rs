use tokio::sync::oneshot;

use log::info;

use crate::{channels::ServerChannels, router, Error};
use config::server::{ConnectionInfo, LaunchConfig, ServerConfig};


/// Settings required to launch a single web server.
#[derive(Debug)]
pub struct ServerSettings {
    /// The underlying server configuration that determines 
    /// ports, virtual hosts and various other server options.
    pub config: ServerConfig,

    /// Channel that receives a notification once the server 
    /// has bound to all ports, usually used to launch a browser.
    pub bind: oneshot::Sender<ConnectionInfo>,

    /// Channel used to shutdown the web server.
    pub shutdown: oneshot::Receiver<bool>,

    /// Channels mapped by host name that the server can use 
    /// to request dynamic rendering of pages (SSR).
    pub channels: ServerChannels,
}

/// Start a server and launch a browser window.
pub async fn launch(
    options: ServerConfig,
    launch: LaunchConfig,
) -> Result<(), Error> {
    // Create a channel to receive the bind address.
    let (ctx, crx) = oneshot::channel::<ConnectionInfo>();

    let (_shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();
    let channels = ServerChannels::new();

    let _ = tokio::task::spawn(async move {
        match crx.await {
            Ok(info) => {
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
            }
            _ => {}
        }
    });

    //println!("{:#?}", options);

    Ok(start(options, ctx, shutdown_rx, channels).await?)
}

/// Start a headless server with the given channels.
pub async fn start(
    options: ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    shutdown: oneshot::Receiver<bool>,
    channels: ServerChannels,
) -> Result<(), Error> {
    Ok(router::serve(options, bind, shutdown, channels).await?)
}
