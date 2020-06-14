use std::net::{SocketAddr, ToSocketAddrs};

use tokio::sync::broadcast::Sender as TokioSender;
use warp::ws::Message;

use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

use log::{info, error};

use open;

use crate::Error;
use crate::server::serve_static;

#[derive(Debug)]
pub struct ServeOptions {
    pub target: PathBuf,
    pub host: String,
    pub port: u16,
    pub open_browser: bool,
    pub watch: Option<PathBuf>,
    pub endpoint: String,
}

pub fn serve_only(options: ServeOptions) -> Result<(), Error> {
    let (tx, _rx) = channel::<(SocketAddr, TokioSender<Message>, String)>();
    serve(options, tx)
}

pub fn serve(options: ServeOptions, bind: Sender<(SocketAddr, TokioSender<Message>, String)>) -> Result<(), Error>
    {

    let address = format!("{}:{}", options.host, options.port);
    let sockaddr: SocketAddr = address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| Error::new(format!("No address found for {}", address)))?;

    let serve_dir = options.target.clone();
    let host = options.host.clone();
    let serve_host = host.clone();
    let open_browser = options.open_browser;

    // A channel used to broadcast to any websockets to reload when a file changes.
    let (tx, _rx) = tokio::sync::broadcast::channel::<Message>(100);
    let reload_tx = tx.clone();

    // Create a channel to receive the bind address.
    let (ctx, crx) = channel::<SocketAddr>();

    let _bind_handle = std::thread::spawn(move || {
        let addr = crx.recv().unwrap();
        let url = format!("http://{}:{}", &host, addr.port());
        //serving_url.foo();
        info!("serve {}", url);
        if open_browser {
            // It is ok if this errors we just don't open a browser window
            open::that(&url).map(|_| ()).unwrap_or(());
        }

        if let Err(e) = bind.send((addr, tx, url)) {
            // FIXME: call out to error_cb
            error!("{}", e);
            std::process::exit(1);
        }
    });

    let endpoint = options.endpoint.clone();
    let thread_handle = std::thread::spawn(move || {
        serve_static::serve(
            serve_dir,
            serve_host,
            endpoint,
            sockaddr,
            ctx,
            reload_tx);
    });

    let _ = thread_handle.join();

    Ok(())
}
