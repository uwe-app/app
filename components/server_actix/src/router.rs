use std::fs::File;
use std::io::BufReader;
use std::pin::Pin;
use std::sync::{atomic::AtomicUsize, Arc};

use serde_json::json;
use url::Url;

use once_cell::sync::OnceCell;

use tokio::sync::oneshot;

use futures::future::ok;
use futures::Future;

use webdav_handler::actix::*;
use webdav_handler::{fakels::FakeLs, localfs::LocalFs, DavConfig, DavHandler};

use actix::Actor;
use actix_files::Files;
use actix_web::{
    dev::{Service, ServiceResponse},
    guard::{self, Guard},
    error,
    http::{
        self,
        header::{self, HeaderValue},
        StatusCode,
    },
    middleware::{DefaultHeaders, NormalizePath, TrailingSlash},
    web, App, HttpRequest, HttpResponse, HttpServer,
};

use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig as TlsServerConfig};

use bracket::Registry;
use log::info;

use crate::{
    channels::{Message, ResponseValue, ServerChannels},
    drop_privileges::{drop_privileges, is_root},
    reload_server::{self, LiveReloadServer},
    websocket::ws_index,
    Error, Result,
};

use config::server::{ConnectionInfo, PortType, ServerConfig};

pub fn parser() -> &'static Registry<'static> {
    static INSTANCE: OnceCell<Registry> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut registry = Registry::new();
        let _ = registry.insert("error", include_str!("error.html"));
        registry
    })
}

pub async fn dav_handler(
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

/// Default route handler.
async fn default_route(req: HttpRequest) -> HttpResponse {
    if req.path() == "" || req.path() == "/" || req.path() == "/index.html" {
        HttpResponse::Ok()
            .content_type("text/html")
            .body(format!("No virtual host matched your request!"))
    } else {
        HttpResponse::NotFound()
            .content_type("text/html")
            .body(format!("No page found!"))
    }
}

#[actix_web::main]
async fn start(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    shutdown: oneshot::Receiver<bool>,
    channels: ServerChannels,
) -> Result<()> {
    let ssl_key = std::env::var("UWE_SSL_KEY").ok();
    let ssl_cert = std::env::var("UWE_SSL_CERT").ok();

    // Allow empty environment variables as a means of disabling SSL certificates
    let use_ssl = if let (Some(key), Some(cert)) =
        (ssl_key.as_ref(), ssl_cert.as_ref())
    {
        !key.is_empty() && !cert.is_empty()
    } else {
        false
    };

    let addr = opts.get_sock_addr(PortType::Infer)?;
    let hosts = opts.hosts();

    // Print each host name here otherwise it would be
    // duplicated for each worker thread if we do it within
    // the HttpServer::new setup closure
    for host in hosts.iter() {
        info!("Host {}", &host.name);
        if let Some(ref webdav) = host.webdav {
            info!("Webdav {}", webdav.directory.display());
        }
    }

    //let data = web::Data::new(Arc::new(Mutex::new(SocketClients { sockets: Vec::new() })));

    let app_state = Arc::new(AtomicUsize::new(0));
    let reload_server = LiveReloadServer::new(app_state.clone()).start();

    let server = HttpServer::new(move || {
        let mut app: App<_, _> = App::new()
            .data(app_state.clone())
            .data(reload_server.clone());

        //.wrap(Logger::default());

        for host in hosts.iter() {
            let disable_cache = host.disable_cache;
            let deny_iframe = host.deny_iframe;
            let redirects =
                host.redirects.clone().unwrap_or(Default::default());

            let watch = host.watch;
            let endpoint = if let Some(ref endpoint) = host.endpoint {
                endpoint.clone()
            } else {
                utils::generate_id(16)
            };

            // Collect all authorities and setup guards for virtual host detection
            let mut host_names = vec![&host.name];
            if let Some(ref authorities) = opts.authorities() {
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
                    channels.websockets.get(&host.name).unwrap().clone();
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
            if let Some(ref webdav) = host.webdav {
                let dav_server = DavHandler::builder()
                    .filesystem(LocalFs::new(
                        webdav.directory.clone(),
                        false,
                        false,
                        false,
                    ))
                    .locksystem(FakeLs::new())
                    .strip_prefix("/webdav")
                    // TODO: support directory listing for webdav?
                    .build_handler();

                app = app.service(
                    web::scope("/webdav")
                        .wrap(NormalizePath::new(TrailingSlash::Always))
                        .guard(guard::Host(&host.name))
                        .data(dav_server)
                        .service(web::resource("/{tail:.*}").to(dav_handler)),
                );
            }

            let live_render_tx = if watch {
                Arc::new(Some(channels.render.get(&host.name).unwrap().clone()))
            } else {
                Arc::new(None)
            };

            app = app.service(
                web::scope("/")
                    // Handle redirect mappings
                    .wrap_fn(move |req, srv| {
                        if let Some(uri) = redirects.get(req.path()) {
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
                                let redirect = if opts.temporary_redirect {
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
                        //println!("Request path: {}", req.path());
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

                                        let res = HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                                            .body(doc);
                                        return Err(
                                            actix_web::Error::from(
                                                error::InternalError::from_response(error, res)))

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
                            .header(
                                header::STRICT_TRANSPORT_SECURITY,
                                "max-age=31536000; includeSubDomains; preload",
                            )
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
                    .service(
                        web::resource(&endpoint)
                            //.guard()
                            .route(web::get().to(ws_index)),
                    )
                    // Serve static files
                    .service(
                        Files::new("", host.directory.clone())
                            .prefer_utf8(true)
                            .index_file(config::INDEX_HTML)
                            .use_etag(!host.disable_cache)
                            .use_last_modified(!host.disable_cache)
                            .redirect_to_slash_directory(),
                    ),
            );
        }

        app = app.default_service(
            // Show something when no virtual hosts match
            // for all requests that are not `GET`.
            web::resource("").route(web::get().to(default_route)).route(
                web::route()
                    .guard(guard::Not(guard::Get()))
                    .to(HttpResponse::MethodNotAllowed),
            ),
        );

        app
    });
    //.workers(4);
    //

    let (server, mut redirect_server) = if use_ssl {
        let key = ssl_key.unwrap();
        let cert = ssl_cert.unwrap();

        let mut config = TlsServerConfig::new(NoClientAuth::new());
        let cert_file = &mut BufReader::new(File::open(&cert)?);
        let key_file = &mut BufReader::new(File::open(&key)?);
        let cert_chain = certs(cert_file)
            .map_err(|_| Error::SslCertChain(cert.to_string()))?;
        let mut keys = pkcs8_private_keys(key_file)
            .map_err(|_| Error::SslPrivateKey(key.to_string()))?;
        config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

        // Always redirect HTTP -> HTTPS
        let http_addr = opts.get_sock_addr(PortType::Insecure)?;
        let tls_port = opts.tls_port();

        let redirect_server = HttpServer::new(move || {
            let mut app: App<_, _> = App::new();
            app = app.service(web::scope("").wrap_fn(move |req, _srv| {
                // This includes any port in the host name!
                let host = req.connection_info().host().to_owned();

                // Must remove the port from the host name
                let host_url: Url = format!("http://{}", host).parse().unwrap();
                let host = host_url.host_str().unwrap();

                let url = if tls_port == 443 {
                    format!("{}//{}", config::SCHEME_HTTPS, host)
                } else {
                    format!("{}//{}:{}", config::SCHEME_HTTPS, host, tls_port)
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
        .bind(http_addr)?;

        (server.bind_rustls(addr, config)?, Some(redirect_server))
    } else {
        (server.bind(addr)?, None)
    };

    let mut addrs = server.addrs();

    if !addrs.is_empty() {
        let tls = opts.tls.is_some();
        let addr = addrs.swap_remove(0);
        let host = opts.default_host.name.clone();
        let info = ConnectionInfo { addr, host, tls };
        bind.send(info)
            .expect("Failed to send web server socket address");
    } else {
        panic!("Could not get web server address!");
    }

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

        futures::try_join!(redirect_server, server)
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

        futures::try_join!(ok(()), server)
    };

    // Propagate errors up the call stack
    let _ = servers?;

    Ok(())
}

pub async fn serve(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    shutdown: oneshot::Receiver<bool>,
    channels: ServerChannels,
) -> Result<()> {
    // Must spawn a new thread as we are already in a tokio runtime
    let handle = std::thread::spawn(move || {
        start(opts, bind, shutdown, channels)
            .expect("Failed to start web server");
    });
    let _ = handle.join();

    Ok(())
}
