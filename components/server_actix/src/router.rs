use std::fs::File;
use std::io::BufReader;

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

use serde::Serialize;

use futures::FutureExt;

use actix_web::{dev::Service, http::header::{self, HeaderValue}, web, guard, middleware, App, HttpServer, Responder};
use actix_files::Files;

use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig as TlsServerConfig};

use bracket::Registry;
use log::{error, info, trace};

use crate::{
    channels::{ResponseValue, ServerChannels},
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


#[actix_web::main]
async fn start(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    mut channels: ServerChannels,
) -> crate::Result<()> {

    let ssl_key = std::env::var("UWE_SSL_KEY");
    let ssl_cert = std::env::var("UWE_SSL_CERT");

    let addr = opts.get_sock_addr(PortType::Infer)?;

    let default_host: &'static HostConfig = &opts.default_host;

    let mut hosts = vec![default_host];
    for host in opts.hosts.iter() {
        hosts.push(host);
    }

    let server = HttpServer::new(move || {
        let mut app: App<_, _> = App::new();
        for host in hosts.iter() {
            let disable_cache = host.disable_cache;

            app = app
                .service(
                    web::scope("")
                        .wrap_fn(move |req, srv| {
                            //println!("Request path: {}", req.path());
                            let fut = srv.call(req);
                            async move {
                                let mut res = fut.await?;
                                if disable_cache {
                                    res.headers_mut().insert(
                                       header::CACHE_CONTROL,
                                       HeaderValue::from_static("no-cache, no-store, must-revalidate"),
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
                            Files::new("/", host.directory.clone())
                                .prefer_utf8(true)
                                .index_file(config::INDEX_HTML)
                                .use_etag(!host.disable_cache)
                                .use_last_modified(!host.disable_cache)
                                .redirect_to_slash_directory()
                        )
                );
        }
        app
    });

    let server = if let (Some(ref key), Some(ref cert)) = (ssl_key.ok(), ssl_cert.ok()) {

        let mut config = TlsServerConfig::new(NoClientAuth::new());
        let cert_file = &mut BufReader::new(File::open(cert).unwrap());
        let key_file = &mut BufReader::new(File::open(key).unwrap());
        let cert_chain = certs(cert_file).unwrap();
        let mut keys = pkcs8_private_keys(key_file).unwrap();
        config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

        server.bind_rustls(addr, config).unwrap()
    } else {
        server.bind(addr).unwrap()
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

    server.run().await?;

    Ok(())
}

pub async fn serve(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    mut channels: ServerChannels,
) -> crate::Result<()> {

    // Must spawn a new thread as we are already in a tokio runtime
    let handle = std::thread::spawn(move || {
        start(opts, bind, channels)
            .expect("Failed to start web server");
    });

    let _ = handle.join();

    Ok(())
}
