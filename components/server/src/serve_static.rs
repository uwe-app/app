use std::path::PathBuf;
use std::net::SocketAddr;
use std::convert::Infallible;

use futures_util::sink::SinkExt;
use futures_util::StreamExt;

use tokio::sync::broadcast;
use tokio::sync::mpsc;

use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};
use warp::http::Uri;
use warp::path::FullPath;
use warp::ws::Message;

use serde::Serialize;

use log::{error, trace};

use config::server::{ServerConfig, PortType};
use crate::Error;

async fn redirect_map(
    path: FullPath,
    opts: ServerConfig,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    if let Some(ref redirects) = opts.redirects {
        if let Some(uri) = redirects.get(path.as_str()) {
            let location = uri.to_string().parse::<Uri>().unwrap();
            return if opts.temporary_redirect {
                Ok(Box::new(warp::redirect::temporary(location)))
            } else {
                Ok(Box::new(warp::redirect(location)))
            };
        }
    }
    Err(warp::reject())
}

async fn redirect_trailing_slash(
    path: FullPath,
    root: PathBuf,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
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
            let location = format!("{}/", path.as_str()).parse::<Uri>().unwrap();
            return Ok(Box::new(warp::redirect(location)));
        }
    }
    Err(warp::reject())
}

pub async fn serve(
    opts: ServerConfig,
    mut bind_tx: mpsc::Sender<(bool, SocketAddr)>,
    reload_tx: broadcast::Sender<Message>) -> crate::Result<()> {

    // TODO: Include the version number from the root crate
    // TODO: sadly, option_env!("CARGO_PKG_VERSION) references 
    // TODO: this crate :(
    let server_id = format!("hypertext");

    let state_opts = opts.clone();

    let address = opts.get_sock_addr(PortType::Infer)?;

    let target = opts.target.clone();
    let state = opts.target.clone();

    let use_tls = opts.tls.is_some();
    let tls = opts.tls.clone();
    let disable_cache = opts.disable_cache;
    let redirect_insecure = opts.redirect_insecure;
    let use_log = opts.log;

    let with_server = warp::reply::with::header("server", &server_id);

    // A warp Filter which captures `reload_tx` and provides an `rx` copy to
    // receive reload messages.
    let sender = warp::any().map(move || reload_tx.subscribe());

    let port = address.clone().port();
    let mut cors = warp::cors().allow_any_origin();
    if port > 0 {
        let scheme = if use_tls {config::SCHEME_HTTPS} else {config::SCHEME_HTTP};
        let origin = format!("{}//{}:{}", scheme, opts.host, port);
        cors = warp::cors()
            .allow_origin(origin.as_str())
            .allow_methods(vec!["GET"]);
    }

    // A warp Filter to handle the livereload endpoint. This upgrades to a
    // websocket, and then waits for any filesystem change notifications, and
    // relays them over the websocket.
    let livereload = warp::path(opts.endpoint.clone())
        .and(warp::ws())
        .and(sender)
        .map(
            move |ws: warp::ws::Ws, mut rx: broadcast::Receiver<Message>| {
                ws.on_upgrade(move |ws| async move {
                    let (mut user_ws_tx, _user_ws_rx) = ws.split();
                    trace!("Websocket got connection");
                    while let Ok(m) = rx.recv().await {
                        let _res = user_ws_tx.send(m).await;
                        //println!("Websocket res {:?}", res);
                    }
                })
            },
        )
        .with(with_server.clone())
        .with(&cors);

    let with_state = warp::any().map(move || state.clone());
    let with_options = warp::any().map(move || state_opts.clone());

    let with_cache_control = warp::reply::with::header(
        "cache-control", "no-cache, no-store, must-revalidate");
    let with_pragma = warp::reply::with::header("pragma", "no-cache");
    let with_expires = warp::reply::with::header("expires", "0");

    let dir_server = warp::fs::dir(target.clone())
        .recover(
            move |e| handle_rejection(e, target.clone())
        );

    let file_server = if disable_cache {
        dir_server
            .with(with_cache_control)
            .with(with_pragma)
            .with(with_expires)
            .with(with_server)
            .boxed()
    } else {
        dir_server
            .with(with_server)
            .boxed()
    };

    let redirect_handler = warp::get()
        .and(warp::path::full())
        .and(with_options)
        .and_then(redirect_map);

    let slash_redirect = warp::get()
        .and(warp::path::full())
        .and(with_state)
        .and_then(redirect_trailing_slash);

    let static_server = redirect_handler.or(slash_redirect).or(file_server);
    let routes = livereload.or(static_server);

    if use_tls {
        let (addr, future) = warp::serve(routes)
            .tls()
            .cert_path(&tls.as_ref().unwrap().cert)
            .key_path(&tls.as_ref().unwrap().key)
            .bind_ephemeral(address);

        bind_tx.try_send((true, addr))
            .expect("Failed to send web server socket address");

        if redirect_insecure {
            super::redirect::spawn(opts.clone()).unwrap_or_else(|_| {
                error!("Failed to start HTTP redirect server");
            });
        }

        future.await;
    } else {
        let bind_result = warp::serve(routes).try_bind_ephemeral(address);
        match bind_result {
            Ok((addr, future)) => {
                if let Err(e) = bind_tx.try_send((false, addr)) {
                    return Err(Error::from(e));
                }
                future.await;
            }
            Err(e) => return Err(Error::from(e))
        }
    }

    Ok(())
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
        if let Ok(content) = utils::fs::read_string(&error_file) {
            return Ok(warp::reply::with_status(warp::reply::html(content), code));
        } else {
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "ERROR_FILE_READ";
        }
    }

    response = warp::reply::html(message.to_string());
    Ok(warp::reply::with_status(response, code))
}
