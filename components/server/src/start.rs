use std::net::SocketAddr;

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use warp::ws::Message;

use log::{error, info};

use crate::Error;
use config::server::{ServerConfig, LaunchConfig, ConnectionInfo};
use super::serve_static;

type WebsocketSender = broadcast::Sender<Message>;
type BindSender = oneshot::Sender<ConnectionInfo>;

pub async fn bind(
    options: ServerConfig,
    launch: LaunchConfig,
    bind: Option<BindSender>,
    channel: Option<WebsocketSender>) -> Result<(), Error> {
    bind_open(options, launch, bind, channel).await
}

async fn bind_open(
    options: ServerConfig,
    launch: LaunchConfig,
    bind: Option<BindSender>,
    channel: Option<WebsocketSender>,
) -> Result<(), Error> {

    // The options are passed down to the web server so 
    // we need to clone this for use on the closure.
    let host = options.host.clone();

    // Create a channel to receive the bind address.
    let (ctx, mut crx) = mpsc::channel::<(bool, SocketAddr)>(100);

    let _ = tokio::task::spawn(async move {
        let (tls, addr) = crx.recv().await.unwrap();
        //let scheme = if tls { config::SCHEME_HTTPS } else { config::SCHEME_HTTP };
        //let url = config::to_url_string(scheme, &host, addr.port());

        let info = ConnectionInfo { addr, host, tls };
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

        if let Some(bind) = bind {
            if let Err(_) = bind.send(info) {
                error!("Failed to notify of server bind event");
                std::process::exit(1);
            }
        }

    });

    serve_static::serve(options, ctx, channel).await?;

    Ok(())
}
