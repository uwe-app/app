use std::net::SocketAddr;
use warp::filters::path::FullPath;
use warp::http::Uri;
use warp::Filter;

use log::info;

use config::server::{PortType, ServerConfig};

use crate::Result;

/// An HTTP server that redirects all requests to HTTPS.
pub fn spawn(options: ServerConfig) -> Result<()> {
    let addr = options.get_sock_addr(PortType::Insecure, None)?;
    let host_url =
        options.get_url(config::SCHEME_HTTPS, PortType::Secure, None);
    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move { run(addr, host_url).await });
    });

    Ok(())
}

async fn run(addr: SocketAddr, base: String) {
    info!(
        "Redirect {} -> {}",
        config::SCHEME_HTTP,
        config::SCHEME_HTTPS
    );
    let redirect =
        warp::any()
            .and(warp::path::full())
            .map(move |path: FullPath| {
                let url = format!("{}{}", base, path.as_str());
                let uri: Uri = url.parse().unwrap();
                warp::redirect(uri)
            });
    warp::serve(redirect).bind(addr).await;
}
