use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::pin::Pin;
use std::sync::{atomic::AtomicUsize, Arc, Mutex};

use serde::Serialize;
use serde_json::json;
use url::Url;

use once_cell::sync::OnceCell;

use tokio::sync::oneshot;

use futures::future::ok;
use futures::Future;

use webdav_handler::actix::*;
use webdav_handler::{fakels::FakeLs, localfs::LocalFs, DavConfig, DavHandler};

use actix::Actor;
use actix_files::{Files, NamedFile};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    error,
    guard::{self, Guard},
    http::{
        self,
        header::{self, HeaderValue},
        StatusCode,
    },
    middleware::{
        Compat, Condition, DefaultHeaders, Logger, NormalizePath, TrailingSlash,
    },
    web, App, HttpRequest, HttpResponse, HttpServer,
};

use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig as TlsServerConfig};

use bracket::Registry;
use log::{error, info, warn};

use crate::{
    channels::{Message, ResponseValue, ServerChannels},
    drop_privileges::{drop_privileges, is_root},
    reload_server::{self, LiveReloadServer},
    websocket::ws_index,
    Error, Result, ServerSettings,
};

use config::{
    memfs::EmbeddedFileSystem,
    server::{ConnectionInfo, PortType, ServerConfig},
};

/// Wrap the default index page in a specific type
/// for the route handler.
pub struct IndexPage(pub String);

/// Wrap the default not found page in a specific type
/// for the route handler.
pub struct NotFoundPage(pub String);

/// Information about known virtual hosts passed to the
/// default index page template.
#[derive(Debug, Serialize)]
struct VirtualHost {
    name: String,
    url: String,
}

fn parser() -> &'static Registry<'static> {
    static INSTANCE: OnceCell<Registry> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut registry = Registry::new();
        let _ = registry.insert("error", include_str!("error.html"));
        let _ = registry
            .insert("default_index", include_str!("default_index.html"));
        let _ = registry.insert(
            "default_not_found",
            include_str!("default_not_found.html"),
        );
        registry
    })
}

async fn dav_handler(
    req: DavRequest,
    davhandler: web::Data<DavHandler>,
) -> DavResponse {
    if let Some(prefix) = req.prefix() {
        let config = DavConfig::new().strip_prefix(prefix);
        davhandler.handle_with(config, req.request).await.into()
    } else {
        davhandler.handle(req.request).await.into()
    }
}

async fn preflight(
    req: HttpRequest,
) -> HttpResponse {

    /*
    println!("Got pre-flight {:?}", req.path());
    println!("Got pre-flight {:#?}", req.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS));
    println!("Got pre-flight {:#?}", req.headers().get(header::ACCESS_CONTROL_REQUEST_METHOD));
    println!("Got pre-flight {:#?}", req.headers().get(header::ORIGIN));
    */

    let mut builder = HttpResponse::Ok();
    if let Some(origin) = req.headers().get(header::ORIGIN) {
        builder.insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, origin));
    }

    if let Some(method) = req.headers().get(header::ACCESS_CONTROL_REQUEST_METHOD) {
        builder.insert_header((header::ACCESS_CONTROL_ALLOW_METHODS, method));
    }

    if let Some(headers) = req.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS) {
        builder.insert_header((header::ACCESS_CONTROL_ALLOW_HEADERS, headers));
    }

    builder.body("")
}

async fn embedded_handler(
    req: HttpRequest,
    memfs: web::Data<Box<dyn EmbeddedFileSystem>>,
) -> HttpResponse {
    let memfs_path = if req.path() == "/" {
        config::INDEX_HTML
    } else {
        req.path().trim_start_matches("/")
    };

    if let Some(memfs_file) = memfs.get(memfs_path) {
        let mime_type = mime_guess::from_path(memfs_path)
            .first()
            .unwrap_or(mime::TEXT_PLAIN);
        HttpResponse::Ok()
            .content_type(mime_type)
            .body(memfs_file.into_owned())
    } else {
        HttpResponse::NotFound()
            .content_type("text/html")
            .body("NOT_FOUND")
    }
}

/// Default route handler.
async fn default_route(
    req: HttpRequest,
    index_page: web::Data<IndexPage>,
    not_found_page: web::Data<NotFoundPage>,
) -> HttpResponse {
    if req.path() == "" || req.path() == "/" || req.path() == "/index.html" {
        HttpResponse::Ok()
            .content_type("text/html")
            .body(&index_page.0)
    } else {
        HttpResponse::NotFound()
            .content_type("text/html")
            .body(&not_found_page.0)
    }
}

#[actix_web::main]
async fn start(
    opts: ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    shutdown: oneshot::Receiver<bool>,
    channels: ServerChannels,
) -> Result<()> {
    let ssl_config = opts.compute_ssl();
    let use_ssl = ssl_config.is_some();

    let addr = opts.get_sock_addr(PortType::Infer)?;
    let mut hosts = opts.hosts().iter().map(|h| h.clone()).collect::<Vec<_>>();

    if hosts.is_empty() {
        return Err(Error::NoVirtualHosts);
    }

    // The first host in the list is the one we send via ConnectionInfo
    // that will be launched in a browser tab
    let host = hosts.get(0).unwrap().name().to_string();

    let temporary_redirect = opts.temporary_redirect();
    let http_addr = opts.get_sock_addr(PortType::Insecure)?;
    let ssl_port = opts.ssl_port();
    let authorities = opts.authorities().clone();

    let mut virtual_hosts = Vec::new();

    // Print each host name here otherwise it would be
    // duplicated for each worker thread if we do it within
    // the HttpServer::new setup closure
    for host in hosts.iter_mut() {
        if host.name().trim().is_empty() {
            return Err(Error::NoVirtualHostName);
        }

        let dir = host.directory().to_string_lossy();
        if dir.is_empty() {
            return Err(Error::NoVirtualHostDirectory(host.name().to_string()));
        }

        if !host.directory().exists() || !host.directory().is_dir() {
            return Err(Error::VirtualHostDirectory(
                host.name().to_string(),
                host.directory().to_path_buf(),
            ));
        }

        if host.redirects().is_none() {
            host.load_redirects()?;
        }

        if host.require_index() {
            let index_page = host.directory().join(config::INDEX_HTML);
            if !index_page.exists() || !index_page.is_file() {
                return Err(Error::NoIndexFile(
                    host.name().to_string(),
                    index_page,
                ));
            }
        }

        info!("Host {} ({})", &host.name(), host.directory().display());
        if let Some(ref webdav) = host.webdav() {
            info!("Webdav {}", webdav.directory().display());
        }

        if let Some(ref endpoint) = host.endpoint() {
            info!("Websocket endpoint {}", endpoint);
        }

        let virtual_host = VirtualHost {
            name: host.name().to_string(),
            url: opts.get_host_url(host.name()),
        };
        virtual_hosts.push(virtual_host);
    }

    let registry = parser();
    let data = json!({
        "hosts": virtual_hosts,
    });

    let default_index = registry.render("default_index", &data).unwrap();
    let default_index = web::Data::new(IndexPage(default_index));

    let default_not_found =
        registry.render("default_not_found", &json!({})).unwrap();
    let default_not_found = web::Data::new(NotFoundPage(default_not_found));

    let app_state = Arc::new(AtomicUsize::new(0));
    let reload_server = LiveReloadServer::new(app_state.clone()).start();

    let host_connections = Arc::new(Mutex::new(HashMap::new()));
    let host_connections_info = Arc::clone(&host_connections);

    let server = HttpServer::new(move || {
        let mut app: App<_, _> = App::new()
            .data(app_state.clone())
            .data(reload_server.clone());

        //.wrap(Logger::default());

        for host in hosts.iter() {
            let disable_cache = host.disable_cache();
            let deny_iframe = host.deny_iframe();
            let log = host.log();
            let redirects =
                host.redirects().clone().unwrap_or(Default::default());
            let error_page = host.directory().join(config::ERROR_HTML);

            let endpoint = host.endpoint().clone();
            let watch = host.endpoint().is_some();

            // Collect all authorities and setup guards for virtual host detection
            let mut host_names = vec![host.name()];
            if let Some(ref authorities) = authorities {
                for name in authorities.iter() {
                    host_names.push(name);
                }
            }
            let host_guards = host_names
                .iter()
                .map(|name| guard::Host(name))
                .collect::<Vec<_>>();

            // Set up logic to broadcast notifications to all connected
            // web sockets when we get a notification via the watch module.
            //
            // Requires a thread per host as we need to block whilst waiting
            // for notifications on the reload channel.
            if watch {
                let broadcast_server = reload_server.clone();
                let reload_rx =
                    channels.websockets.get(host.name()).unwrap().clone();
                let mut live_reload_rx = reload_rx.subscribe();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        while let Ok(m) = live_reload_rx.recv().await {
                            match m {
                                Message::Text(message) => {
                                    broadcast_server.do_send(
                                        reload_server::Message(message),
                                    );
                                }
                            }
                        }
                    });
                });
            }

            // Setup webdav route
            if let Some(ref webdav) = host.webdav() {
                let dav_server = DavHandler::builder()
                    .filesystem(LocalFs::new(
                        webdav.directory().to_path_buf(),
                        false,
                        false,
                        false,
                    ))
                    .locksystem(FakeLs::new())
                    .strip_prefix(webdav.mount_path())
                    .autoindex(webdav.listing())
                    //.indexfile()
                    .build_handler();

                app = app.service(
                    web::scope(webdav.mount_path())
                        .wrap(NormalizePath::new(TrailingSlash::Always))
                        .wrap(Condition::new(log, Compat::new(Logger::default())))
                        .wrap(
                            // Access-Control-Max-Age: 86400
                            DefaultHeaders::new()
                                .header(
                                    header::SERVER,
                                    config::generator::user_agent(),
                                )
                                .header(header::REFERRER_POLICY, "origin")
                                .header(header::ACCESS_CONTROL_MAX_AGE, "86400")
                                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                                .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Depth, Content-Type")
                                .header(header::ACCESS_CONTROL_ALLOW_METHODS, "POST, GET, OPTIONS, PUT, PATCH, PROPFIND")
                        )
                        .guard(guard::Host(host.name()))
                        .data(dav_server)
                        .route("/{tail:.*}", web::method(http::Method::OPTIONS).to(preflight))
                        .service(web::resource("/{tail:.*}").to(dav_handler)),
                );
            }

            let live_render_tx = if watch {
                Arc::new(Some(
                    channels.render.get(host.name()).unwrap().clone(),
                ))
            } else {
                Arc::new(None)
            };

            if let Some(ref embedded) = host.embedded() {
                app = app.service(
                    web::resource("/{tail:.*}")
                        .data(embedded.clone())
                        .route(web::get().to(embedded_handler))
                );

            } else {

                if let Some(ref endpoint) = endpoint {
                    {
                        let mut endpoints = host_connections_info.lock().unwrap();
                        endpoints.insert(host.name().to_string(), endpoint.clone());
                    }

                    app = app.service(
                        web::resource(endpoint)
                            .route(web::get().to(ws_index)),
                    );
                }

                app = app.service(
                    web::scope("/")
                        // Handle redirect mappings
                        .wrap_fn(move |req, srv| {
                            if let Some(uri) = redirects.items().get(req.path()) {
                                let location = uri.to_string();

                                let response: Pin<
                                    Box<
                                        dyn Future<
                                            Output = std::result::Result<
                                                ServiceResponse,
                                                actix_web::Error,
                                            >,
                                        >,
                                    >,
                                > = Box::pin(async move {
                                    let redirect = if temporary_redirect {
                                        HttpResponse::TemporaryRedirect()
                                            .append_header((
                                                http::header::LOCATION,
                                                location,
                                            ))
                                            .finish()
                                            .into_body()
                                    } else {
                                        HttpResponse::PermanentRedirect()
                                            .append_header((
                                                http::header::LOCATION,
                                                location,
                                            ))
                                            .finish()
                                            .into_body()
                                    };

                                    Ok(req.into_response(redirect))
                                });

                                return response;
                            }

                            srv.call(req)
                        })
                        // Handle conditional headers
                        .wrap_fn(move |req, srv| {
                            let fut = srv.call(req);
                            async move {
                                let mut res = fut.await?;
                                if disable_cache {
                                    res.headers_mut().insert(
                                        header::CACHE_CONTROL,
                                        HeaderValue::from_static(
                                            "no-cache, no-store, must-revalidate",
                                        ),
                                    );
                                    res.headers_mut().insert(
                                        header::PRAGMA,
                                        HeaderValue::from_static("no-cache"),
                                    );
                                    res.headers_mut().insert(
                                        header::EXPIRES,
                                        HeaderValue::from_static("0"),
                                    );
                                }

                                if deny_iframe {
                                    res.headers_mut().insert(
                                        header::X_FRAME_OPTIONS,
                                        HeaderValue::from_static("DENY"),
                                    );
                                }

                                Ok(res)
                            }
                        })
                        // Handle live rendering
                        .wrap_fn(move |req, srv| {
                            let mut href = if req.path().ends_with("/")
                                || req.path().ends_with(".html")
                            {
                                if req.path().ends_with("/") && req.path() != "/" {
                                    Some(format!(
                                        "{}{}",
                                        req.path(),
                                        config::INDEX_HTML
                                    ))
                                } else {
                                    Some(req.path().to_string())
                                }
                            } else {
                                None
                            };

                            let tx = Arc::clone(&live_render_tx);
                            let fut = srv.call(req);
                            async move {
                                if let (Some(href), Some(ref tx)) =
                                    (href.take(), &*tx)
                                {
                                    let (resp_tx, resp_rx) =
                                        oneshot::channel::<ResponseValue>();

                                    // TODO: handle RenderSendError
                                    let _ = tx.send((href, resp_tx)).await;

                                    // WARN: currently this will serve from a stale
                                    // WARN: cache if the live render channel fails.
                                    if let Ok(response) = resp_rx.await {
                                        if let Some(error) = response {
                                            let registry = parser();
                                            let data = json!({
                                                "title": "Render Error",
                                                "message": error.to_string()});
                                            let doc = registry
                                                .render("error", &data)
                                                .unwrap();

                                            let res = HttpResponse::build(
                                                StatusCode::INTERNAL_SERVER_ERROR,
                                            )
                                            .body(doc);
                                            return Err(actix_web::Error::from(
                                                error::InternalError::from_response(
                                                    error, res,
                                                ),
                                            ));
                                        }
                                    }
                                }

                                Ok(fut.await?)
                            }
                        })
                        // Always add these headers
                        .wrap(
                            DefaultHeaders::new()
                                .header(
                                    header::SERVER,
                                    config::generator::user_agent(),
                                )
                                .header(header::REFERRER_POLICY, "origin")
                                .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
                                .header(header::X_XSS_PROTECTION, "1; mode=block")

                                /*
                                .header(
                                    header::STRICT_TRANSPORT_SECURITY,
                                    "max-age=31536000; includeSubDomains; preload",
                                )
                                */
                                // TODO: allow configuring this header
                                .header("permissions-policy", "geolocation=()"),
                        )
                        // Check virtual hosts
                        .guard(guard::fn_guard(move |req| {
                            for g in host_guards.iter() {
                                if g.check(req) {
                                    return true;
                                }
                            }
                            false
                        }))
                        .wrap(Condition::new(log, Compat::new(Logger::default())))
                        // Serve static files
                        .service(
                            Files::new("", host.directory().to_path_buf())
                                .default_handler(move |req: ServiceRequest| {
                                    let err = error_page.clone();
                                    let (http_req, _payload) = req.into_parts();
                                    async {
                                        let response = if err.exists() {
                                            match NamedFile::open(err) {
                                                Ok(file) => {
                                                    file.into_response(&http_req)
                                                }
                                                Err(e) => {
                                                    return Err(
                                                        actix_web::Error::from(e),
                                                    )
                                                }
                                            }
                                        } else {
                                            // TODO: pretty not found when no 404.html for the host?
                                            HttpResponse::NotFound()
                                                .content_type("text/html")
                                                .body("NOT_FOUND")
                                        };

                                        Ok(ServiceResponse::new(http_req, response))
                                    }
                                })
                                .prefer_utf8(true)
                                .index_file(config::INDEX_HTML)
                                .use_etag(!host.disable_cache())
                                .use_last_modified(!host.disable_cache())
                                .redirect_to_slash_directory(),
                        ),
                );

            }
        }

        app = app.default_service(
            // Show something when no virtual hosts match
            // for all requests that are not `GET`.
            web::resource("")
                .app_data(default_index.clone())
                .app_data(default_not_found.clone())
                .route(web::get().to(default_route))
                .route(
                    web::route()
                        .guard(guard::Not(guard::Get()))
                        .to(HttpResponse::MethodNotAllowed),
                ),
        );

        app
    })
    .workers(opts.workers());

    let (mut server, mut redirect_server) = if let Some(ref ssl_config) =
        ssl_config
    {
        let cert = ssl_config.cert();
        let key = ssl_config.key();

        let mut config = TlsServerConfig::new(NoClientAuth::new());
        let cert_file = &mut BufReader::new(
            File::open(cert)
                .map_err(|_| Error::SslCertFile(cert.to_path_buf()))?,
        );
        let key_file = &mut BufReader::new(
            File::open(key)
                .map_err(|_| Error::SslKeyFile(key.to_path_buf()))?,
        );
        let cert_chain = certs(cert_file)
            .map_err(|_| Error::SslCertChain(cert.to_path_buf()))?;

        let mut keys = pkcs8_private_keys(key_file)
            .map_err(|_| Error::SslPrivateKey(key.to_path_buf()))?;

        if keys.is_empty() {
            return Err(Error::SslKeyRead(key.to_path_buf()));
        }

        config.set_single_cert(cert_chain, keys.remove(0))?;

        let redirect_server = if opts.redirect_insecure() {
            // Always redirect HTTP -> HTTPS
            let redirect_server = HttpServer::new(move || {
                let mut app: App<_, _> = App::new();
                app = app.service(web::scope("").wrap_fn(move |req, _srv| {
                    // This includes any port in the host name!
                    let host = req.connection_info().host().to_owned();

                    // Must remove the port from the host name
                    let host_url: Url =
                        format!("http://{}", host).parse().unwrap();
                    let host = host_url.host_str().unwrap();

                    let url = if ssl_port == 443 {
                        format!("{}//{}", config::SCHEME_HTTPS, host)
                    } else {
                        format!(
                            "{}//{}:{}",
                            config::SCHEME_HTTPS,
                            host,
                            ssl_port
                        )
                    };

                    let url = format!("{}{}", url, req.uri().to_owned());
                    ok(req.into_response(
                        HttpResponse::MovedPermanently()
                            .append_header((http::header::LOCATION, url))
                            .finish()
                            .into_body(),
                    ))
                }));
                app
            })
            .disable_signals()
            .workers(opts.workers())
            .bind(http_addr)?;

            Some(redirect_server)
        } else {
            None
        };

        (server.bind_rustls(addr, config)?, redirect_server)
    } else {
        (server.bind(addr)?, None)
    };

    if opts.disable_signals() {
        server = server.disable_signals();
    }

    let mut addrs = server.addrs();

    let host_endpoints = Arc::clone(&host_connections);

    let bind_notify = async move {
        if !addrs.is_empty() {

            let endpoints = {
                let endpoints = host_endpoints.lock().unwrap();
                endpoints.clone()
            };

            let addr = addrs.swap_remove(0);
            let info = ConnectionInfo::new(addr, host, use_ssl, endpoints);
            match bind.send(info) {
                Err(_) => {
                    warn!("Failed to send connection info on bind channel");
                }
                _ => {}
            }
        } else {
            warn!("Could not send connection info to bind channel (server address not available)");
        }
        Ok(())
    };

    if is_root() {
        drop_privileges()?;
    }

    // Support redirect server when running over SSL
    let servers = if let Some(redirect_server) = redirect_server.take() {
        let server = server.run();
        let redirect_server = redirect_server.run();
        let shutdown_server = server.clone();
        let shutdown_redirect_server = redirect_server.clone();

        // Must spawn a thread for the shutdown handler otherwise
        // it prevents Ctrl-c from quitting as the shutdown future
        // will block the current thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                match shutdown.await {
                    Ok(graceful) => {
                        shutdown_redirect_server.stop(graceful).await;
                        shutdown_server.stop(graceful).await;
                    }
                    _ => {}
                }
            });
        });

        futures::try_join!(redirect_server, server, bind_notify)
    } else {
        let server = server.run();
        let shutdown_server = server.clone();

        // Must spawn a thread for the shutdown handler otherwise
        // it prevents Ctrl-c from quitting as the shutdown future
        // will block the current thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                match shutdown.await {
                    Ok(graceful) => {
                        shutdown_server.stop(graceful).await;
                    }
                    _ => {}
                }
            });
        });

        futures::try_join!(ok(()), server, bind_notify)
    };

    // Propagate errors up the call stack
    let _ = servers?;

    Ok(())
}

pub async fn serve(settings: impl Into<Vec<ServerSettings>>) -> Result<()> {
    let settings = settings.into();
    let length = settings.len();

    for (i, settings) in settings.into_iter().enumerate() {
        let last = i == length - 1;

        // Must spawn a new thread as we are already in a tokio runtime
        let handle = std::thread::spawn(move || {
            if let Err(e) = start(
                settings.config,
                settings.bind,
                settings.shutdown,
                settings.channels,
            ) {
                match e {
                    Error::Io(ref e) => {
                        if e.kind() == std::io::ErrorKind::AddrInUse {
                            let delimiter = utils::terminal::delimiter();
                            eprintln!("{}", delimiter);
                            warn!("Could not start the server because the address is being used!");
                            warn!("This happens when a server is already running on a port.");
                            warn!("");
                            warn!("To fix this problem either stop the existing server or choose ");
                            warn!("a different port for the web server using the --port ");
                            warn!("and --ssl-port options.");
                            eprintln!("{}", delimiter);
                        }
                    }
                    _ => {}
                }

                error!("{}", e);
                std::process::exit(1);
            }
        });

        // Block on the last thread we spawn to keep
        // this process alive
        if last {
            let _ = handle.join();
        }
    }

    Ok(())
}
