use std::net::SocketAddr;
use std::path::PathBuf;
use std::convert::Infallible;

use futures_util::sink::SinkExt;
use futures_util::StreamExt;

use tokio::sync::broadcast;
use tokio::sync::oneshot;

use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};
use warp::filters::BoxedFilter;
use warp::http::Uri;
use warp::path::FullPath;
use warp::ws::Message;

use serde::Serialize;

use log::{error, trace};

use config::server::{ServerConfig, HostConfig, PortType, ConnectionInfo};
use crate::Error;

macro_rules! bind {
    (
        $opts:expr,
        $routes:expr,
        $addr:expr,
        $bind_tx:expr
    ) => {

        let host = $opts.default_host.name.clone();
        let use_tls = $opts.tls.is_some();
        let redirect_insecure = $opts.redirect_insecure;
        if use_tls {
            let (addr, future) = warp::serve($routes)
                .tls()
                .cert_path(&$opts.tls.as_ref().unwrap().cert)
                .key_path(&$opts.tls.as_ref().unwrap().key)
                .bind_ephemeral(*$addr);

            let info = ConnectionInfo {addr, host, tls: true};
            $bind_tx.send(info)
                .expect("Failed to send web server socket address");

            if redirect_insecure {
                super::redirect::spawn($opts.clone()).unwrap_or_else(|_| {
                    error!("Failed to start HTTP redirect server");
                });
            }

            future.await;
        } else {
            let bind_result = warp::serve($routes).try_bind_ephemeral(*$addr);
            match bind_result {
                Ok((addr, future)) => {
                    let info = ConnectionInfo {addr, host, tls: true};
                    $bind_tx.send(info)
                        .expect("Failed to send web server socket address");
                    future.await;
                }
                Err(e) => return Err(Error::from(e))
            }
        }
    };
}

fn for_host(
    address: &SocketAddr,
    host: &'static HostConfig) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    let hostname = &format!("{}:{}", host.name, address.port());
    warp::host::exact(hostname)
}


fn get_fallback(
    address: &SocketAddr) -> BoxedFilter<(impl Reply,)> {
    let hostname = &format!("{}", address.to_string());
    warp::host::exact(hostname)
        .map(|| "Fallback")
        .boxed()
}

macro_rules! router {
    (
        $address:expr,
        $opts:expr,
        $routes:expr,
        $bind_tx:expr
    ) => {
        let default_host: &'static HostConfig = &$opts.default_host;
        let with_server = get_with_server($opts);
        let all = $routes.with(with_server)
            // TODO: use a different rejection handler that returns
            // TODO: internal system error pages
            .recover(move |e| handle_rejection(e, default_host.directory.clone()));

        if let Some(ref log) = default_host.log {
            bind!($opts, all.with(warp::log(&log.prefix)), $address, $bind_tx);
        } else {
            bind!($opts, all, $address, $bind_tx);
        }
    };
}

macro_rules! for_host {
    (
        $opts:expr,
        $address:expr,
        $host:expr,
        $reload_tx:expr
    ) => {
        {
            let static_server = get_static_server($opts, $host);
            let hostname = &format!("{}:{}", $host.name, $address.port());
            let reload_tx = if let Some(reload_tx) = $reload_tx.clone() {
                reload_tx 
            } else {
                let (ws_tx, _) = broadcast::channel::<Message>(0);
                ws_tx
            };
            let livereload = get_live_reload($opts, reload_tx).unwrap();

            // NOTE: We would like to conditionally add the livereload route
            // NOTE: but spent so much time trying to fight the warp type 
            // NOTE: system to achieve it and failing it is much easier 
            // NOTE: to just make it a noop. :(
            warp::host::exact(hostname)
                .and(livereload.or(static_server))
                .boxed()
        }
    }
}

macro_rules! server2 {
    (
        $address:expr,
        $opts:expr,
        $bind_tx:expr,
        $reload_tx:expr
    ) => {
        //let fallback = get_fallback(&$address);
        let default_host: &'static HostConfig = &$opts.default_host;
        let default_host_route = for_host!($opts, $address, default_host, $reload_tx);

        if !$opts.hosts.is_empty() {
            let mut routes = vec![default_host_route];
            let mut host_routes = $opts.hosts.iter()
                .map(|h| for_host!($opts, $address, h, $reload_tx))
                .collect::<Vec<_>>();
            routes.append(&mut host_routes);
            let hosts_routes = warp::fold(routes).unwrap();
            router!($address, $opts, hosts_routes, $bind_tx);
        } else {
            router!($address, $opts, default_host_route, $bind_tx);
        }
    }
}

macro_rules! server {
    (
        $address:expr,
        $opts:expr,
        $routes:expr,
        $bind_tx:expr
    ) => {

        //let never = warp::path("never-match-this-thin").map(|| "Ooops").and($routes);

        let fallback = get_fallback(&$address);
        let default_host: &'static HostConfig = &$opts.default_host;
        let default_host_route = fallback.or(
            for_host(&$address, default_host).and($routes));

        if !$opts.hosts.is_empty() {
            let routes = $opts.hosts.iter()
                .map(|h| for_host(&$address, default_host).and($routes))
                .collect::<Vec<_>>();
            //let all_host_routes = routes.iter()
                //.fold(default_host_route, |acc, r| acc.or(*r));
            //routes.foo();
        } else {
            router!($address, $opts, default_host_route, $bind_tx);
        }

        //let all = hosts_routes.with(with_server)
            //// TODO: use a different rejection handler that returns
            //// TODO: internal system error pages
            //.recover(move |e| handle_rejection(e, default_host.directory.clone()));

        //if let Some(ref log) = default_host.log {
            //bind!($opts, all.with(warp::log(&log.prefix)), $address, $bind_tx);
        //} else {
            //bind!($opts, all, $address, $bind_tx);
        //}
    };
}

async fn redirect_map(
    path: FullPath,
    opts: &'static ServerConfig,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    if let Some(ref redirects) = opts.default_host.redirects {
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

fn get_with_server(_opts: &ServerConfig) -> warp::filters::reply::WithHeader {
    let server_id = format!("hypertext/{}", config::app::version(None));
    warp::reply::with::header("server", &server_id)
}

fn get_static_server(opts: &'static ServerConfig, host: &'static HostConfig) -> BoxedFilter<(impl Reply,)> {

    // NOTE: Later we add this to all requests in the macro 
    // NOTE: but we also need to add it here so the `else` branch
    // NOTE: below for `disable_cache` has a type that matches the 
    // NOTE: `if` branch. A `noop` filter would be really useful 
    // NOTE: here but warp does not expose the functionality to create one.
    let with_server = get_with_server(opts);

    let disable_cache = host.disable_cache;

    let with_cache_control = warp::reply::with::header(
        "cache-control", "no-cache, no-store, must-revalidate");
    let with_pragma = warp::reply::with::header("pragma", "no-cache");
    let with_expires = warp::reply::with::header("expires", "0");

    let dir_server = warp::fs::dir(host.directory.clone())
        .recover(move |e| handle_rejection(e, host.directory.clone()));

    let file_server = if disable_cache {
        dir_server
            .with(with_cache_control)
            .with(with_pragma)
            .with(with_expires)
            .with(with_server.clone())
            .boxed()
    } else {
        dir_server
            .with(with_server.clone())
            .boxed()
    };

    let with_options = warp::any().map(move || opts);
    let redirect_handler = warp::get()
        .and(warp::path::full())
        .and(with_options)
        .and_then(redirect_map);

    let with_target = warp::any().map(move || host.directory.clone());
    let slash_redirect = warp::get()
        .and(warp::path::full())
        .and(with_target)
        .and_then(redirect_trailing_slash);

    let static_server = redirect_handler.or(slash_redirect).or(file_server);
    static_server.boxed()
}

fn get_live_reload(
    opts: &ServerConfig,
    reload_tx: broadcast::Sender<Message>) -> crate::Result<BoxedFilter<(impl Reply,)>> {

    let use_tls = opts.tls.is_some();

    let address = opts.get_sock_addr(PortType::Infer)?;
    let port = address.port();
    let mut cors = warp::cors().allow_any_origin();
    if port > 0 {
        let scheme = if use_tls {config::SCHEME_HTTPS} else {config::SCHEME_HTTP};
        let origin = format!("{}//{}:{}", scheme, opts.host, port);
        cors = warp::cors()
            .allow_origin(origin.as_str())
            .allow_methods(vec!["GET"]);
    }

    // A warp Filter which captures `reload_tx` and provides an `rx` copy to
    // receive reload messages.
    let sender = warp::any().map(move || reload_tx.subscribe());

    // A warp Filter to handle the livereload endpoint. This upgrades to a
    // websocket, and then waits for any filesystem change notifications, and
    // relays them over the websocket.
    let livereload = warp::path(opts.default_host.endpoint.as_ref().unwrap().clone())
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
        .with(&cors);

    Ok(livereload.boxed())
}

pub async fn serve(
    opts: &'static ServerConfig,
    bind_tx: oneshot::Sender<ConnectionInfo>,
    reload_tx: Option<broadcast::Sender<Message>>
) -> crate::Result<()> {

    let addr = opts.get_sock_addr(PortType::Infer)?;

    server2!(&addr, opts, bind_tx, reload_tx);

    //let static_server = get_static_server(opts, &opts.default_host);
    //if let Some(reload_tx) = reload_tx {
        //let livereload = get_live_reload(opts, reload_tx)?;
        //server!(&addr, opts, livereload.clone().or(static_server.clone()), bind_tx);
    //} else {
        //server!(&addr, opts, static_server.clone(), bind_tx);
    //}

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
        //eprintln!("unhandled rejection: {:?}", err);
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
