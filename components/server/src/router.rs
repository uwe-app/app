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
        Compat, Condition, DefaultHeaders, Logger, 
    },
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer,
};

use rustls::{Certificate, PrivateKey, ServerConfig as TlsServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};

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
            .body(index_page.0.to_string())
    } else {
        HttpResponse::NotFound()
            .content_type("text/html")
            .body(not_found_page.0.to_string())
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

    let mut host_connections = HashMap::new();

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
            let has_root_redirect =
                if let Some(ref redirects) = host.redirects() {
                    redirects.items().contains_key("/")
                } else {
                    false
                };
            if (!index_page.exists() || !index_page.is_file())
                && !has_root_redirect
            {
                return Err(Error::NoIndexFile(
                    host.name().to_string(),
                    index_page,
                ));
            }
        }

        info!("Host {} ({})", &host.name(), host.directory().display());

        if let Some(ref endpoint) = host.endpoint() {
            info!("Websocket endpoint {}", endpoint);
            host_connections
                .insert(host.name().to_string(), endpoint.to_string());
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

    let app_state = Arc::new(AtomicUsize::new(0));
    let reload_server = LiveReloadServer::new(app_state.clone()).start();

    let broadcast_started = Arc::new(Mutex::new(false));

    let server = HttpServer::new(move || {

        let default_index = registry.render("default_index", &data).unwrap();
        let default_index = Data::new(IndexPage(default_index));

        let default_not_found =
            registry.render("default_not_found", &json!({})).unwrap();
        let default_not_found = Data::new(NotFoundPage(default_not_found));


        let mut app: App<_> = App::new()
            .app_data(default_index)
            .app_data(default_not_found)
            .app_data(Data::new(app_state.clone()))
            .app_data(Data::new(reload_server.clone()));

        let broadcast_start = Arc::clone(&broadcast_started);

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
                let mut started = broadcast_start.lock().unwrap();
                if !&*started {
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
                                        //println!("Watch thread got notification {:?}", message);
                                        broadcast_server.do_send(
                                            reload_server::Message(message),
                                        );
                                    }
                                }
                            }
                        });
                    });
                    *started = true;
                }
            }
            
            /*
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
                                .add((
                                    header::SERVER,
                                    config::generator::user_agent(),
                                ))
                                .add((header::REFERRER_POLICY, "origin"))
                                .add((header::ACCESS_CONTROL_MAX_AGE, "86400"))
                                .add((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
                                .add((header::ACCESS_CONTROL_ALLOW_HEADERS, "Depth, Content-Type"))
                                .add((header::ACCESS_CONTROL_ALLOW_METHODS, "POST, GET, OPTIONS, PUT, PATCH, PROPFIND, MKCOL"))
                        )
                        .guard(guard::Host(host.name()))
                        .app_data(Data::new(dav_server))
                        .route("/{tail:.*}", web::method(http::Method::OPTIONS).to(preflight))
                        .service(web::resource("/{tail:.*}").to(dav_handler)),
                );
            }
            */

            let live_render_tx = if watch {
                Arc::new(Some(
                    channels.render.get(host.name()).unwrap().clone(),
                ))
            } else {
                Arc::new(None)
            };

            // Handle serving embedded file system used for the editor
            if let Some(ref embedded) = host.embedded() {
                app = app.service(
                    web::resource("/{tail:.*}")
                        .app_data(Data::new(embedded.clone()))
                        .route(web::get().to(embedded_handler))
                );

            // Normal virtual host configuration
            } else {

                // Endpoint for a websocket server
                if let Some(ref endpoint) = endpoint {
                    app = app.service(
                        web::resource(endpoint)
                            .route(web::get().to(ws_index)),
                    );
                }

                app = app.service(
                    web::scope("")
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
                                    } else {
                                        HttpResponse::PermanentRedirect()
                                            .append_header((
                                                http::header::LOCATION,
                                                location,
                                            ))
                                            .finish()
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
                            {
                                let mut headers = DefaultHeaders::new()
                                    .add((
                                        header::SERVER,
                                        config::generator::user_agent(),
                                    ))
                                    .add((header::REFERRER_POLICY, "origin"))
                                    .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
                                    .add((header::X_XSS_PROTECTION, "1; mode=block"))

                                    /*
                                    .header(
                                        header::STRICT_TRANSPORT_SECURITY,
                                        "max-age=31536000; includeSubDomains; preload",
                                    )
                                    */
                                    // TODO: allow configuring this header
                                    .add(("permissions-policy", "geolocation=()"));

                                if watch {
                                    headers = headers.add((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"));
                                }

                                headers
                            }
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
                            Files::new("/", host.directory().to_path_buf())
                                .default_handler(move |req: ServiceRequest| {
                                    let err = error_page.clone();
                                    let (http_req, _payload) = req.into_parts();
                                    async {
                                        let response = if err.exists() {
                                            match NamedFile::open(err) {
                                                Ok(file) => {
                                                    let file = file.set_status_code(StatusCode::NOT_FOUND);
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

        app.default_service(web::get().to(default_route))
    })
    .workers(opts.workers());

    let (mut server, mut redirect_server) = if let Some(ref ssl_config) =
        ssl_config
    {
        let cert = ssl_config.cert();
        let key = ssl_config.key();

        let cert_file = &mut BufReader::new(
            File::open(cert)
                .map_err(|_| Error::SslCertFile(cert.to_path_buf()))?,
        );
        let key_file = &mut BufReader::new(
            File::open(key)
                .map_err(|_| Error::SslKeyFile(key.to_path_buf()))?,
        );

        let cert_chain =
            certs(cert_file)?.into_iter().map(Certificate).collect();

        let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)?
            .into_iter()
            .map(PrivateKey)
            .collect();

        if keys.is_empty() {
            return Err(Error::SslKeyRead(key.to_path_buf()));
        }

        let config = TlsServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, keys.remove(0))?;

        let redirect_server = if opts.redirect_insecure() {
            // Always redirect HTTP -> HTTPS
            let redirect_server = HttpServer::new(move || {
                let mut app: App<_> = App::new();
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
                            .finish(),
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

    let bind_notify = async move {
        if !addrs.is_empty() {
            let addr = addrs.swap_remove(0);
            let info = ConnectionInfo::new(
                addr,
                host,
                use_ssl,
                host_connections.clone(),
            );
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
        let shutdown_server = server.handle();
        let shutdown_redirect_server = redirect_server.handle();

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
        let handle = server.handle();

        // Must spawn a thread for the shutdown handler otherwise
        // it prevents Ctrl-c from quitting as the shutdown future
        // will block the current thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                match shutdown.await {
                    Ok(graceful) => {
                        handle.stop(graceful).await;
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
