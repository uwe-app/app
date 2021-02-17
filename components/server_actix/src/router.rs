use std::fs::File;
use std::io::BufReader;

use url::Url;

use once_cell::sync::OnceCell;

use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

use futures::future::ok;

use webdav_handler::actix::*;
use webdav_handler::{fakels::FakeLs, localfs::LocalFs, DavConfig, DavHandler};

use actix_files::Files;
use actix_web::{
    dev::{Service},
    guard,
    http::{self, header::{self, HeaderValue}},
    middleware, web, App, HttpServer, HttpResponse,
};

use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig as TlsServerConfig};

use bracket::Registry;
use log::info;

use crate::{
    channels::{ResponseValue, ServerChannels},
    //drop_privileges::*,
    Result,
    Error,
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

pub async fn dav_handler(req: DavRequest, davhandler: web::Data<DavHandler>) -> DavResponse {
    if let Some(prefix) = req.prefix() {
        let config = DavConfig::new().strip_prefix(prefix);
        davhandler.handle_with(config, req.request).await.into()
    } else {
        davhandler.handle(req.request).await.into()
    }
}

#[actix_web::main]
async fn start(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    mut channels: ServerChannels,
) -> Result<()> {

    let ssl_key = std::env::var("UWE_SSL_KEY");
    let ssl_cert = std::env::var("UWE_SSL_CERT");

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

    let server = HttpServer::new(move || {
        let mut app: App<_, _> = App::new();
        for host in hosts.iter() {

            let disable_cache = host.disable_cache;

            if let Some(ref webdav) = host.webdav {
                let dav_server = DavHandler::builder()
                        .filesystem(LocalFs::new(webdav.directory.clone(), false, false, false))
                        .locksystem(FakeLs::new())
                        .strip_prefix("/webdav")
                        .build_handler();

                app = app.service(
                    web::scope("/webdav")
                        .wrap(middleware::NormalizePath::new(middleware::TrailingSlash::Always))
                        .guard(guard::Host(&host.name))
                        .data(dav_server)
                        .service(web::resource("/{tail:.*}").to(dav_handler))
                ); 
            }

            app = app.service(
                web::scope("/")
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
                            Ok(res)
                        }
                    })
                    .guard(guard::Host(&host.name))
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
        app
    });
    //.workers(4);

    let (server, mut redirect_server) = if let (Some(ref key), Some(ref cert)) =
        (ssl_key.ok(), ssl_cert.ok())
    {
        let mut config = TlsServerConfig::new(NoClientAuth::new());
        let cert_file = &mut BufReader::new(File::open(cert)?);
        let key_file = &mut BufReader::new(File::open(key)?);
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
                app = app.service(
                    web::scope("")
                        .wrap_fn(move |req, _srv| {
                            // This includes any port in the host name!
                            let host = req.connection_info().host().to_owned();

                            // Must remove the port from the host name
                            let host_url: Url = format!("http://{}", host).parse().unwrap();
                            let host = host_url.host_str().unwrap();

                            let url = if tls_port == 443 {
                                format!("{}//{}", config::SCHEME_HTTPS, host)
                            } else {
                                format!(
                                    "{}//{}:{}",
                                    config::SCHEME_HTTPS,
                                    host,
                                    tls_port
                                )
                            };

                            let url = format!("{}{}", url, req.uri().to_owned());
                            ok(req.into_response(
                                HttpResponse::MovedPermanently()
                                    .append_header((http::header::LOCATION, url))
                                    .finish()
                                    .into_body(),
                            ))
                        })
                );
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

    let servers = if let Some(redirect_server) = redirect_server.take() {
        futures::join!(redirect_server.run(), server.run())
    } else {
        futures::join!(server.run(), ok(()))
    };

    // Propagate errors up the call stack
    match servers {
        (s1, s2) => {
            let _ = s1?; 
            let _ = s2?; 
        }
    }

    Ok(())
}

pub async fn serve(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    mut channels: ServerChannels,
) -> Result<()> {
    // Must spawn a new thread as we are already in a tokio runtime
    let handle = std::thread::spawn(move || {
        start(opts, bind, channels).expect("Failed to start web server");
    });

    let _ = handle.join();

    Ok(())
}
