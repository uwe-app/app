use std::convert::Infallible;
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use serde_json::json;

use futures::future;
use futures_util::sink::SinkExt;
use futures_util::StreamExt;

use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

use warp::filters::BoxedFilter;
use warp::http::{StatusCode, Uri};
use warp::path::FullPath;
use warp::reject::Reject;
use warp::ws::Message;
use warp::{Filter, Rejection, Reply};

use webdav_handler::warp::dav_dir;

use serde::Serialize;

use bracket::Registry;
use log::{error, info, trace};

use crate::{
    channels::{ResponseValue, ServerChannels},
    helper_routes::host_ephemeral,
    drop_privileges::*,
    Error,
};
use config::server::{ConnectionInfo, HostConfig, PortType, ServerConfig};

pub fn parser() -> &'static Registry<'static> {
    static INSTANCE: OnceCell<Registry> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut registry = Registry::new();
        let _ = registry.insert("error", include_str!("error.html"));
        registry
    })
}

#[derive(Debug)]
struct RenderSendError;

impl Reject for RenderSendError {}

struct OptFmt<T>(Option<T>);

impl<T: fmt::Display> fmt::Display for OptFmt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref t) = self.0 {
            fmt::Display::fmt(t, f)
        } else {
            f.write_str("-")
        }
    }
}

macro_rules! virtual_hosts {
    (
        $opts:expr,
        $filters:expr,
        $addr:expr,
        $bind:expr,
        $shutdown_rx:expr
    ) => {
        // NOTE: This mess is because `warp` cannot dynamically chain filters using
        // NOTE: `or()`; we can't use macro_rules!() as it is runtime data and
        // NOTE: because `or()` wraps with the `Or` struct it is impossible to type
        // NOTE: this in a loop :(
        if $filters.is_empty() {
            panic!("No virtual hosts!");
        } else if $filters.len() == 1 {
            let all = $filters.swap_remove(0);
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 2 {
            let all = $filters.swap_remove(0).or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 3 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 4 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 5 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 6 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 7 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 8 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 9 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 10 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 11 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 12 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 13 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 14 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 15 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else if $filters.len() == 16 {
            let all = $filters
                .swap_remove(0)
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0))
                .or($filters.swap_remove(0));
            bind!($opts, all, $addr, $bind, $shutdown_rx);
        } else {
            panic!("Too many virtual hosts!");
        }

    };
}

macro_rules! bind {
    (
        $opts:expr,
        $routes:expr,
        $addr:expr,
        $bind_channel:expr,
        $shutdown_rx:expr
    ) => {
        let with_server = get_with_server($opts);
        let host = $opts.default_host.name.clone();
        let tls = $opts.tls.is_some();
        let redirect_insecure = $opts.redirect_insecure;
        let routes = $routes
            .recover(move |e| handle_rejection(e, None))
            .with(with_server);

        if tls {
            let (addr, future) = warp::serve(routes)
                .tls()
                .cert_path(&$opts.tls.as_ref().unwrap().cert)
                .key_path(&$opts.tls.as_ref().unwrap().key)
                .bind_with_graceful_shutdown(*$addr, async {
                    $shutdown_rx.await.ok();
                });

            info!("Bind TLS {}", addr);

            if redirect_insecure {
                super::redirect::spawn($opts.clone()).unwrap_or_else(|_| {
                    error!("Failed to start HTTP redirect server");
                });
            }

            if is_root() {
                drop_privileges()?;
            }

            let info = ConnectionInfo { addr, host, tls };
            $bind_channel
                .send(info)
                .expect("Failed to send web server socket address");

            future.await;
        } else {
            let bind_result = warp::serve(routes)
                .try_bind_with_graceful_shutdown(*$addr, async {
                    $shutdown_rx.await.ok();
                });

            match bind_result {
                Ok((addr, future)) => {
                    info!("Bind {}", addr);

                    if is_root() {
                        drop_privileges()?;
                    }

                    let info = ConnectionInfo { addr, host, tls };
                    $bind_channel
                        .send(info)
                        .expect("Failed to send web server socket address");

                    future.await;
                }
                Err(e) => return Err(Error::from(e)),
            }
        }
    };
}

fn get_host_filter(
    address: &SocketAddr,
    opts: &'static ServerConfig,
    host: &'static HostConfig,
) -> BoxedFilter<(impl Reply,)> {
    let (hostname, static_server) = get_static_server(address, opts, host);

    let none = warp::path::end()
        .and_then(|| {
            future::err(warp::reject::not_found())
        });

    if let Some(ref webdav) = host.webdav {
        info!("Webdav {}", webdav.directory.display());
        let dav_server = warp::path("-")
            .and(warp::path("webdav"))
            .and(dav_dir(&webdav.directory, false, webdav.listing));
        host_ephemeral(&hostname, opts.authorities()).and(dav_server.or(static_server)).boxed()
    } else {
        host_ephemeral(&hostname, opts.authorities()).and(none.or(static_server)).boxed()
    }
}

fn get_host_filter_watch(
    address: &SocketAddr,
    opts: &'static ServerConfig,
    host: &'static HostConfig,
    channels: &mut ServerChannels,
) -> BoxedFilter<(impl Reply,)> {
    let (hostname, static_server) = get_static_server(address, opts, host);

    let livereload = get_live_reload(opts, host, channels).unwrap();
    let request_tx = channels.render.get(&host.name).unwrap().clone();
    let request = warp::any().map(move || request_tx.clone());

    let response_arc = Arc::new(RwLock::new(
        channels.render_responses.remove(&host.name).unwrap(),
    ));
    let response = warp::any().map(move || Arc::clone(&response_arc));

    let live_renderer = warp::any()
        .and(warp::path::full())
        .and(request)
        .and(response)
        .and_then(live_render);

    let none = warp::path::end()
        .and_then(|| {
            future::err(warp::reject::not_found())
        });

    if let Some(ref webdav) = host.webdav {
        info!("Webdav {}", webdav.directory.display());
        let dav_server = warp::path("-")
            .and(warp::path("webdav"))
            .and(dav_dir(&webdav.directory, false, webdav.listing));

        if let Some(ref editor_directory) = host.editor_directory {
            let editor_server = warp::path("-")
                .and(warp::path("editor"))
                .and(warp::fs::dir(editor_directory.clone()));

            host_ephemeral(&hostname, opts.authorities())
                .and(editor_server.or(dav_server).or(livereload.or(live_renderer).or(static_server)))
                .boxed()
        } else {
            let editor_server = warp::path("-")
                .and(warp::path("editor"))
                .and_then(|| {
                    future::err(warp::reject::not_found())
                });

            host_ephemeral(&hostname, opts.authorities())
                .and(editor_server.or(dav_server).or(livereload.or(live_renderer).or(static_server)))
                .boxed()
        }
    } else {
        host_ephemeral(&hostname, opts.authorities())
            .and(none.or(livereload.or(live_renderer).or(static_server)))
            .boxed()
    }
}

fn get_live_reload(
    opts: &ServerConfig,
    host: &'static HostConfig,
    channels: &mut ServerChannels,
) -> crate::Result<BoxedFilter<(impl Reply,)>> {
    let reload_tx = channels.websockets.get(&host.name).unwrap().clone();

    let use_tls = opts.tls.is_some();

    let address = opts.get_sock_addr(PortType::Infer)?;
    let port = address.port();
    let mut cors = warp::cors().allow_any_origin();
    if port > 0 {
        let scheme = if use_tls {
            config::SCHEME_HTTPS
        } else {
            config::SCHEME_HTTP
        };
        let origin = format!("{}//{}:{}", scheme, &host.name, port);
        cors = warp::cors()
            .allow_origin(origin.as_str())
            .allow_methods(vec!["GET"]);
    }

    // A warp Filter which captures `reload_tx` and provides an `rx` copy to
    // receive reload messages.
    let sender = warp::any().map(move || reload_tx.subscribe());

    let endpoint = if let Some(ref endpoint) = host.endpoint {
        endpoint.clone()
    } else {
        utils::generate_id(16)
    };

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

async fn live_render(
    path: FullPath,
    tx: mpsc::Sender<String>,
    rx: Arc<RwLock<mpsc::Receiver<ResponseValue>>>,
) -> Result<impl Reply, warp::Rejection> {
    if path.as_str().ends_with("/") || path.as_str().ends_with(".html") {
        // NOTE: the link for the root page is stored as `/` not `/index.html` at the moment.
        let href = if path.as_str().ends_with("/") && path.as_str() != "/" {
            format!("{}{}", path.as_str(), config::INDEX_HTML)
        } else {
            path.as_str().to_string()
        };

        let _ = tx
            .send(href)
            .await
            .map_err(|_| warp::reject::custom(RenderSendError))?;

        // Wait for a reply to that we block before serving
        // the compiled static asset otherwise changes will
        // not be reflected!
        let mut reply_channel = rx.write().await;

        if let Some(response) = reply_channel.recv().await {
            if let Some(error) = response {
                let registry = parser();
                let data = json!({"title": "Render Error", "message": error.to_string()});
                let doc = registry.render("error", &data).unwrap();
                return Ok(warp::reply::with_status(
                    warp::reply::html(doc),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
        }
    }
    Err(warp::reject())
}

async fn redirect_map(
    path: FullPath,
    opts: &'static ServerConfig,
    host: &'static HostConfig,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    if let Some(ref redirects) = host.redirects {
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
            let location =
                format!("{}/", path.as_str()).parse::<Uri>().unwrap();
            return Ok(Box::new(warp::redirect(location)));
        }
    }
    Err(warp::reject())
}

fn get_with_server(_opts: &ServerConfig) -> warp::filters::reply::WithHeader {
    warp::reply::with::header("server", config::generator::user_agent())
}

fn get_static_server(
    address: &SocketAddr,
    opts: &'static ServerConfig,
    host: &'static HostConfig,
) -> (String, BoxedFilter<(impl Reply,)>) {
    let disable_cache = host.disable_cache;

    let with_cache_control = warp::reply::with::header(
        "cache-control",
        "no-cache, no-store, must-revalidate",
    );

    let with_pragma = warp::reply::with::header("pragma", "no-cache");
    let with_expires = warp::reply::with::header("expires", "0");

    let referrer_policy = warp::reply::with::header(
        "referrer-policy",
        "origin");
    let x_content_type_options = warp::reply::with::header(
        "x-content-type-options",
        "nosniff");
    let x_frame_options = warp::reply::with::header(
        "x-frame-options",
        "DENY");
    let x_xss_protection = warp::reply::with::header(
        "x-xss-protection",
        "1; mode=block");

    let permissions_policy = warp::reply::with::header(
        "permissions-policy",
        "geolocation=()");

    let strict_transport_security = warp::reply::with::header(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains; preload");

    // TODO: Content-Security-Policy
    // SEE: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Feature-Policy
    // TODO: Support setting Permissions-Policy values

    let dir_server = warp::fs::dir(host.directory.clone())
        .recover(move |e| handle_rejection(e, Some(host.directory.clone())));

    let file_server = if disable_cache {
        dir_server
            .with(referrer_policy)
            .with(x_content_type_options)
            .with(x_frame_options)
            .with(x_xss_protection)
            .with(permissions_policy)
            .with(strict_transport_security)
            .with(with_cache_control)
            .with(with_pragma)
            .with(with_expires)
            .boxed()
    } else {
        dir_server
            .with(referrer_policy)
            .with(x_content_type_options)
            .with(x_frame_options)
            .with(x_xss_protection)
            .with(permissions_policy)
            .with(strict_transport_security)
            .boxed()
    };

    let with_options = warp::any().map(move || opts);
    let with_host = warp::any().map(move || host);
    let redirect_handler = warp::get()
        .and(warp::path::full())
        .and(with_options)
        .and(with_host)
        .and_then(redirect_map);

    let with_target = warp::any().map(move || host.directory.clone());
    let slash_redirect = warp::get()
        .and(warp::path::full())
        .and(with_target)
        .and_then(redirect_trailing_slash);

    let host_state = host.clone();
    let log = warp::log::custom(move |info| {
        if host_state.log {
            log::info!(
                target: &host_state.name,
                "{} \"{} {} {:?}\" {} \"{}\" \"{}\" {:?}",
                OptFmt(info.remote_addr()),
                info.method(),
                info.path(),
                info.version(),
                info.status().as_u16(),
                OptFmt(info.referer()),
                OptFmt(info.user_agent()),
                info.elapsed(),
            );
        }
    });

    let static_server = redirect_handler
        .or(slash_redirect)
        .or(file_server)
        .with(log);

    let port = address.port();
    let host_port = format!("{}:{}", host.name, port);
    let hostname: &str = if port == 80 || port == 443 {
        &host.name
    } else {
        &host_port
    };
    (hostname.to_string(), static_server.boxed())
}

pub async fn serve(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    mut channels: ServerChannels,
) -> crate::Result<()> {
    let addr = opts.get_sock_addr(PortType::Infer)?;
    let default_host: &'static HostConfig = &opts.default_host;
    let should_watch = default_host.watch;

    let mut configs = vec![default_host];
    for host in opts.hosts.iter() {
        configs.push(host);
    }

    if should_watch {
        let mut filters: Vec<BoxedFilter<_>> = configs
            .iter()
            .map(|c| get_host_filter_watch(&addr, opts, c, &mut channels))
            .collect();

        virtual_hosts!(opts, filters, &addr, bind, channels.shutdown);
    } else {
        let mut filters: Vec<BoxedFilter<_>> = configs
            .iter()
            .map(|c| get_host_filter(&addr, opts, c))
            .collect();

        virtual_hosts!(opts, filters, &addr, bind, channels.shutdown);
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
async fn handle_rejection(
    err: Rejection,
    directory: Option<PathBuf>,
) -> Result<impl Reply, Infallible> {
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

    let response;

    if let Some(root) = directory {
        let mut error_file = root.clone();
        error_file.push(format!("{}.html", code.as_u16()));
        if error_file.exists() {
            if let Ok(content) = utils::fs::read_string(&error_file) {
                return Ok(warp::reply::with_status(
                    warp::reply::html(content),
                    code,
                ));
            } else {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "ERROR_FILE_READ";
            }
        }
    }

    response = warp::reply::html(message.to_string());
    Ok(warp::reply::with_status(response, code))
}
