// Code derived from: https://github.com/rust-lang/mdBook/blob/master/src/cmd/serve.rs
// Respect to the original authors.
//
// Modified to gracefully handle ephemeral port.

#[cfg(feature = "watch")]
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::sync::broadcast;
use warp::http::StatusCode;
use warp::{Filter, Reply, Rejection};

use std::convert::Infallible;
use open;

use serde::{Serialize};

use tokio::sync::broadcast::Sender as TokioSender;
use warp::ws::Message;

use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

use crate::{utils, Error};
use log::{info, trace, error};

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
        .ok_or_else(|| Error::new(format!("no address found for {}", address)))?;

    let build_dir = options.target.clone();
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
            error!("{}", e);
            std::process::exit(1);
        }
    });

    let endpoint = options.endpoint.clone();
    let thread_handle = std::thread::spawn(move || {
        serve_web(
            build_dir,
            serve_host,
            endpoint,
            sockaddr,
            ctx,
            reload_tx);
    });

    let _ = thread_handle.join();

    Ok(())
}

#[tokio::main]
async fn serve_web(
    build_dir: PathBuf,
    host: String,
    endpoint: String,
    address: SocketAddr,
    bind_tx: Sender<SocketAddr>,
    reload_tx: broadcast::Sender<Message>) {

    // A warp Filter which captures `reload_tx` and provides an `rx` copy to
    // receive reload messages.
    let sender = warp::any().map(move || reload_tx.subscribe());

    let port = address.port();
    let mut cors = warp::cors().allow_any_origin();
    if port > 0 {
        let origin = format!("http://{}:{}", host, port);
        cors = warp::cors()
            .allow_origin(origin.as_str())
            .allow_methods(vec!["GET"]);
    }

    // A warp Filter to handle the livereload endpoint. This upgrades to a
    // websocket, and then waits for any filesystem change notifications, and
    // relays them over the websocket.
    let livereload = warp::path(endpoint)
        .and(warp::ws())
        .and(sender)
        .map(move |ws: warp::ws::Ws, mut rx: broadcast::Receiver<Message>| {
            ws.on_upgrade(move |ws| async move {
                let (mut user_ws_tx, _user_ws_rx) = ws.split();
                trace!("websocket got connection");
                if let Ok(m) = rx.recv().await {
                    trace!("notify of reload");
                    let _ = user_ws_tx.send(m).await;
                }
            })
        })
        .with(&cors);

    let root = build_dir.clone();

    // TODO: support server logging!

    let static_route = warp::fs::dir(build_dir)
        .recover(move |e| handle_rejection(e, root.clone()));
        //.with(warp::log("static"));

    let routes = livereload.or(static_route);

    let bind_result = warp::serve(routes).try_bind_ephemeral(address);
    match bind_result {
        Ok((addr, future)) => {
            if let Err(e) = bind_tx.send(addr) {
                error!("{}", e);
                std::process::exit(1);
            }
            future.await;
        },
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}

// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
async fn handle_rejection(err: Rejection, root: PathBuf) -> Result<impl Reply, Infallible> {
    let mut code;
    let mut message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        // We can handle a specific error, here METHOD_NOT_ALLOWED,
        // and render it however we want
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        // We should have expected this... Just log and say its a 500
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let mut error_file = root.clone();
    error_file.push(format!("{}.html", code.as_u16()));
    let response;
    if error_file.exists() {
        if let Ok(content) = utils::read_string(&error_file) {
            return Ok(warp::reply::with_status(warp::reply::html(content), code))
        } else {
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "ERROR_FILE_READ";
        }

    }

    response = warp::reply::html(message.to_string());
    Ok(warp::reply::with_status(response, code))
}
