// Code derived from: https://github.com/rust-lang/mdBook/blob/master/src/cmd/serve.rs
// Respect to the original authors.

#[cfg(feature = "watch")]
//use super::watch;
use crate::{open};
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use tokio::sync::broadcast;
use warp::ws::Message;
use warp::Filter;

use crate::Error;
use log::{info, trace};

#[derive(Debug)]
pub struct ServeOptions {
    pub target: PathBuf,
    pub host: String,
    pub port: String,
    pub open_browser: bool,
}

// The HTTP endpoint for the websocket used to trigger reloads when a file changes.
const LIVE_RELOAD_ENDPOINT: &str = "__livereload";

pub fn serve(options: ServeOptions) -> Result<(), Error> {

    let address = format!("{}:{}", options.host, options.port);
    let sockaddr: SocketAddr = address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| Error::new(format!("no address found for {}", address)))?;

    let build_dir = options.target;

    // A channel used to broadcast to any websockets to reload when a file changes.
    let (tx, _rx) = tokio::sync::broadcast::channel::<Message>(100);

    let reload_tx = tx.clone();
    let thread_handle = std::thread::spawn(move || {
        serve_web(build_dir, sockaddr, reload_tx);
    });

    let serving_url = format!("http://{}", address);
    info!("serve {}", serving_url);

    if options.open_browser {
        open::that(serving_url);
    }

    //#[cfg(feature = "watch")]
    //watch::trigger_on_change(&book, move |paths, book_dir| {
        //info!("Files changed: {:?}", paths);
        //info!("Building book...");

        //// FIXME: This area is really ugly because we need to re-set livereload :(
        //let result = MDBook::load(&book_dir)
            //.and_then(|mut b| {
                //b.config
                    //.set("output.html.livereload-url", &livereload_url)?;
                //Ok(b)
            //})
            //.and_then(|b| b.build());

        //if let Err(e) = result {
            //error!("Unable to load the book");
            //utils::log_backtrace(&e);
        //} else {
            //let _ = tx.send(Message::text("reload"));
        //}
    //});

    let _ = thread_handle.join();

    Ok(())
}

#[tokio::main]
async fn serve_web(build_dir: PathBuf, address: SocketAddr, reload_tx: broadcast::Sender<Message>) {
    // A warp Filter which captures `reload_tx` and provides an `rx` copy to
    // receive reload messages.
    let sender = warp::any().map(move || reload_tx.subscribe());

    // A warp Filter to handle the livereload endpoint. This upgrades to a
    // websocket, and then waits for any filesystem change notifications, and
    // relays them over the websocket.
    let livereload = warp::path(LIVE_RELOAD_ENDPOINT)
        .and(warp::ws())
        .and(sender)
        .map(|ws: warp::ws::Ws, mut rx: broadcast::Receiver<Message>| {
            ws.on_upgrade(move |ws| async move {
                let (mut user_ws_tx, _user_ws_rx) = ws.split();
                trace!("websocket got connection");
                if let Ok(m) = rx.recv().await {
                    trace!("notify of reload");
                    let _ = user_ws_tx.send(m).await;
                }
            })
        });
    // A warp Filter that serves from the filesystem.
    let book_route = warp::fs::dir(build_dir);
    let routes = livereload.or(book_route);
    warp::serve(routes).run(address).await;
}
