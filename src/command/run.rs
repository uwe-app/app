use std::net::SocketAddr;

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use warp::ws::Message;

use log::{error, info};

use crate::Error;
use config::server::{ServerConfig, LaunchConfig};
use server::serve_static;

pub async fn serve_only(options: ServerConfig, launch: LaunchConfig) -> Result<(), Error> {
    let (ws_tx, _rx) = broadcast::channel::<Message>(100);
    let (tx, _rx) = oneshot::channel::<(SocketAddr, String, bool)>();
    serve(options, launch, ws_tx, tx).await
}

pub async fn serve(
    options: ServerConfig,
    launch: LaunchConfig,
    ws_notify: broadcast::Sender<Message>,
    bind: oneshot::Sender<(SocketAddr, String, bool)>,
) -> Result<(), Error> {

    let host = options.host.clone();
    let open_browser = launch.open;

    // Create a channel to receive the bind address.
    let (ctx, mut crx) = mpsc::channel::<(bool, SocketAddr)>(100);

    let _ = tokio::task::spawn(async move {
        let (tls, addr) = crx.recv().await.unwrap();
        let scheme = if tls { config::SCHEME_HTTPS } else { config::SCHEME_HTTP };
        let url = config::to_url_string(scheme, &host, addr.port());
        info!("Serve {}", url);

        if open_browser {
            // It is ok if this errors we just don't open a browser window
            open::that(&url).map(|_| ()).unwrap_or(());
        }

        if let Err(_) = bind.send((addr, url, tls)) {
            error!("Failed to notify of server bind event");
            std::process::exit(1);
        }
    });

    serve_static::serve(options, ctx, ws_notify).await?;

    Ok(())
}
