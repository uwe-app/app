use tokio::sync::oneshot;

use log::info;

use crate::{channels::ServerChannels, router, Error};
use config::server::{ConnectionInfo, LaunchConfig, ServerConfig};

/// Settings required to launch a single web server.
#[derive(Debug)]
pub struct ServerSettings {
    /// The underlying server configuration that determines
    /// ports, virtual hosts and various other server options.
    pub config: ServerConfig,

    /// Channel that receives a notification once the server
    /// has bound to all ports, usually used to launch a browser.
    pub bind: oneshot::Sender<ConnectionInfo>,

    /// Channel used to shutdown the web server.
    pub shutdown: oneshot::Receiver<bool>,

    /// Channels mapped by host name that the server can use
    /// to request dynamic rendering of pages (SSR).
    pub channels: ServerChannels,
}

impl Into<Vec<ServerSettings>> for ServerSettings {
    fn into(self) -> Vec<ServerSettings> {
        vec![self]
    }
}

/// Start a server and call a callback once the
/// server is bound and connection info is available.
pub async fn open<F>(config: ServerConfig, callback: F) -> Result<(), Error>
where
    F: Fn(ConnectionInfo) + Send + 'static,
{
    // Create a channel to receive the bind address.
    let (bind, crx) = oneshot::channel::<ConnectionInfo>();
    let (_shutdown_tx, shutdown) = oneshot::channel::<bool>();
    let channels = ServerChannels::new();

    /*
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            match crx.await {
                Ok(info) => (callback)(info),
                _ => {}
            }
        });
    });
    */

    let _ = tokio::task::spawn(async move {
        match crx.await {
            Ok(info) => (callback)(info),
            _ => {}
        }
    });

    Ok(start(ServerSettings {
        config,
        bind,
        shutdown,
        channels,
    })
    .await?)
}

/// Start a server and launch a browser window.
pub async fn launch(
    config: ServerConfig,
    launch: LaunchConfig,
) -> Result<(), Error> {
    Ok(open(config, move |info| {
        let url = info.to_url();
        info!("Serve {}", &url);

        // Most of the time we want to open a browser unless explictly
        // disabled however in the case of the live reload logic it
        // takes control of opening the browser so that:
        //
        // 1) Don't start to compile until we have bound to a port.
        // 2) Don't open a browser window unless the build succeeds.
        //
        if launch.open {
            // It is ok if this errors we just don't open a browser window
            open::that(&url).map(|_| ()).unwrap_or(());
        }
    })
    .await?)
}

/// Start a headless server using the supplied server configurations.
pub async fn run(configs: Vec<ServerConfig>) -> Result<(), Error> {
    let mut servers = Vec::new();
    let mut bind_receivers = Vec::new();

    for config in configs {
        let (bind, bind_rx) = oneshot::channel::<ConnectionInfo>();
        let (_shutdown_tx, shutdown) = oneshot::channel::<bool>();
        let channels = ServerChannels::new();
        bind_receivers.push(bind_rx);
        servers.push(ServerSettings {
            config,
            bind,
            shutdown,
            channels,
        });
    }
    Ok(start(servers).await?)
}

/// Start a headless server with the given channels.
pub async fn start(
    settings: impl Into<Vec<ServerSettings>>,
) -> Result<(), Error> {
    Ok(router::serve(settings).await?)
}
