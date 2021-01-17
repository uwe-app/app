use std::net::SocketAddr;
use warp::filters::path::FullPath;
use warp::host::Authority;
use warp::http::Uri;
use warp::Filter;

use log::info;

use config::server::{PortType, ServerConfig};

use crate::Result;

/// An HTTP server that redirects all requests to HTTPS.
pub fn spawn(options: ServerConfig) -> Result<()> {
    let addr = options.get_sock_addr(PortType::Insecure, None)?;
    let tls_port = options.tls_port();

    let host_url =
        options.get_url(config::SCHEME_HTTPS, PortType::Secure, None);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move { run(addr, host_url, tls_port).await });
    });

    Ok(())
}

async fn run(addr: SocketAddr, base: String, tls_port: u16) {
    info!(
        "Redirect {} -> {}",
        config::SCHEME_HTTP,
        config::SCHEME_HTTPS
    );
    let redirect = warp::any()
        .and(warp::path::full())
        .and(warp::host::optional())
        .map(move |path: FullPath, authority: Option<Authority>| {
            let host_url = if let Some(authority) = authority {
                if tls_port == 443 {
                    format!("{}//{}", config::SCHEME_HTTPS, authority.host())
                } else {
                    format!(
                        "{}//{}:{}",
                        config::SCHEME_HTTPS,
                        authority.host(),
                        tls_port
                    )
                }
            } else {
                base.clone()
            };
            let url = format!("{}{}", host_url, path.as_str());
            let uri: Uri = url.parse().unwrap();
            warp::redirect(uri)
        });
    warp::serve(redirect).bind(addr).await;
}
