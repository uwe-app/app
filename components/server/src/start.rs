use std::net::SocketAddr;

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use warp::ws::Message;

use log::{error, info};

use crate::Error;
use config::server::{ServerConfig, LaunchConfig};
use super::serve_static;

pub struct ServerChannel {
    /// Notification channel for websocket messages.
    pub websocket: broadcast::Sender<Message>,
    /// Notification sent when a server binds successfully.
    pub bind: oneshot::Sender<(SocketAddr, String, bool)>,
}

impl Default for ServerChannel {
    fn default() -> Self {
        let (ws_tx, _rx) = broadcast::channel::<Message>(100);
        let (bind_tx, _rx) = oneshot::channel::<(SocketAddr, String, bool)>();
        Self {
            websocket: ws_tx,
            bind: bind_tx,
        }
    }
}

pub async fn bind(
    options: ServerConfig,
    launch: LaunchConfig,
    channel: Option<ServerChannel>) -> Result<(), Error> {

    let channel = if let Some(channel) = channel {
        channel
    } else { Default::default() };

    bind_open(options, launch, channel).await
}

async fn bind_open(
    options: ServerConfig,
    launch: LaunchConfig,
    channel: ServerChannel,
) -> Result<(), Error> {

    let ws = channel.websocket.clone();

    // The options are passed down to the web server so 
    // we need to clone this for use on the closure.
    let host = options.host.clone();

    // Create a channel to receive the bind address.
    let (ctx, mut crx) = mpsc::channel::<(bool, SocketAddr)>(100);

    let _ = tokio::task::spawn(async move {
        let (tls, addr) = crx.recv().await.unwrap();
        let scheme = if tls { config::SCHEME_HTTPS } else { config::SCHEME_HTTP };
        let url = config::to_url_string(scheme, &host, addr.port());
        info!("Serve {}", url);

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

        if let Err(_) = channel.bind.send((addr, url, tls)) {
            error!("Failed to notify of server bind event");
            std::process::exit(1);
        }
    });

    serve_static::serve(options, ctx, ws).await?;

    Ok(())
}
