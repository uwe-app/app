use std::collections::HashMap;
use std::path::PathBuf;
use std::net::{SocketAddr, ToSocketAddrs};

use tokio::sync::mpsc::channel;
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use warp::http::Uri;
use warp::ws::Message;

use log::{error, info};

use server::{serve_static, WebServerOptions};
use crate::Error;

#[derive(Debug)]
pub struct ServeOptions {
    pub target: PathBuf,
    pub host: String,
    pub port: u16,
    pub open_browser: bool,
    pub watch: Option<PathBuf>,
    pub endpoint: String,
    pub redirects: Option<HashMap<String, Uri>>,
}

pub async fn serve_only(options: ServeOptions) -> Result<(), Error> {
    let (tx, _rx) = mpsc::channel::<(SocketAddr, broadcast::Sender<Message>, String)>(100);
    serve(options, tx).await
}

pub async fn serve(
    options: ServeOptions,
    mut bind: mpsc::Sender<(SocketAddr, broadcast::Sender<Message>, String)>,
) -> Result<(), Error> {

    let address = format!("{}:{}", options.host, options.port);
    let sockaddr: SocketAddr = address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| Error::NoSocketAddress(address))?;

    let serve_dir = options.target.clone();
    let host = options.host.clone();
    let serve_host = host.clone();
    let open_browser = options.open_browser;

    // A channel used to broadcast to any websockets to reload when a file changes.
    let (tx, _rx) = broadcast::channel::<Message>(100);
    let reload_tx = tx.clone();

    // Create a channel to receive the bind address.
    let (ctx, mut crx) = channel::<SocketAddr>(100);

    let _ = tokio::task::spawn(async move {
        let addr = crx.recv().await.unwrap();
        let url = format!("http://{}:{}", &host, addr.port());
        //serving_url.foo();
        info!("serve {}", url);
        if open_browser {
            // It is ok if this errors we just don't open a browser window
            open::that(&url).map(|_| ()).unwrap_or(());
        }

        if let Err(e) = bind.try_send((addr, tx, url)) {
            // FIXME: call out to error_cb
            error!("{}", e);
            std::process::exit(1);
        }
    });

    let web_server_opts = WebServerOptions {
        serve_dir,
        endpoint: options.endpoint.clone(),
        host: serve_host,
        address: sockaddr,
        redirects: options.redirects,
        log: true,
        temporary_redirect: true,
    };

    serve_static::serve(web_server_opts, ctx, reload_tx).await;

    Ok(())
}
