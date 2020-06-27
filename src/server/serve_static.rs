// Code derived from: https://github.com/rust-lang/mdBook/blob/master/src/cmd/serve.rs
// Respect to the original authors.
//
// Modified to gracefully handle ephemeral port.

#[cfg(feature = "watch")]
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use std::net::SocketAddr;
use tokio::sync::broadcast;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

use std::convert::Infallible;

use serde::Serialize;

use warp::ws::Message;
use warp::path::FullPath;

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::utils;
use log::{error, trace};

async fn redirect_trailing_slash(root: PathBuf, path: FullPath) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let mut req = path.as_str();
    if req != "/" && !req.ends_with("/") {
        // Need to remove the trailing slash so the path
        // is not treated as absolute
        req = req.trim_start_matches("/");

        // Convert to file system path separators
        let file_path = utils::url::to_path_separator(req);
        let mut buf = root.clone();
        buf.push(file_path);
        if buf.is_dir() {
            let location = format!("{}/", path.as_str()).parse::<warp::http::Uri>().unwrap();
            return Ok(Box::new(warp::redirect(location)))
        }
    }
    Err(warp::reject())
}

#[tokio::main]
pub async fn serve(
    serve_dir: PathBuf,
    host: String,
    endpoint: String,
    address: SocketAddr,
    bind_tx: Sender<SocketAddr>,
    reload_tx: broadcast::Sender<Message>,
) {
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
        .map(
            move |ws: warp::ws::Ws, mut rx: broadcast::Receiver<Message>| {
                ws.on_upgrade(move |ws| async move {
                    let (mut user_ws_tx, _user_ws_rx) = ws.split();
                    trace!("websocket got connection");
                    if let Ok(m) = rx.recv().await {
                        trace!("notify of reload");
                        let _ = user_ws_tx.send(m).await;
                    }
                })
            },
        )
        .with(&cors);

    let root = serve_dir.clone();

    let state = serve_dir.clone();
    let with_state = warp::any().map(move || state.clone());


    let file_server = warp::fs::dir(serve_dir)
        .recover(move |e| handle_rejection(e, root.clone()));

    let slash_redirect = warp::get()
        .and(with_state)
        .and(warp::path::full())
        .and_then(redirect_trailing_slash)
        .or(file_server);

    // TODO: support server logging!
    //.with(warp::log("static"));

    //let static_routes = livereload.or(static_route);

    let routes = livereload.or(slash_redirect);

    let bind_result = warp::serve(routes).try_bind_ephemeral(address);
    match bind_result {
        Ok((addr, future)) => {
            if let Err(e) = bind_tx.send(addr) {
                error!("{}", e);
                std::process::exit(1);
            }
            future.await;
        }
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
            return Ok(warp::reply::with_status(warp::reply::html(content), code));
        } else {
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "ERROR_FILE_READ";
        }
    }

    response = warp::reply::html(message.to_string());
    Ok(warp::reply::with_status(response, code))
}
