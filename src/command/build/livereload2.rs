use std::path::Path;

use log::{debug, error, info};

use tokio::sync::broadcast;
use tokio::sync::oneshot;
use warp::ws::Message;

use notify::DebouncedEvent::{Create, Remove, Rename, Write};
use notify::RecursiveMode::Recursive;
use notify::Watcher;
use std::thread::sleep;
use std::time::Duration;

use compiler::Compiler;
use compiler::parser::Parser;
use config::ProfileSettings;
use config::server::{ServerConfig, HostConfig, ConnectionInfo};

use server::{Channels, HostChannel};

use crate::{Error, ErrorCallback};
use super::invalidator::Invalidator;

pub async fn start<P: AsRef<Path>>(
    project: P,
    args: &mut ProfileSettings,
    error_cb: ErrorCallback,
) -> Result<(), Error> {

    // Compile the project
    let result = workspace::compile(project, args).await?;

    // Create a channel to receive the bind address.
    let (bind_tx, bind_rx) = oneshot::channel::<ConnectionInfo>();

    // Create the collection of channels
    let mut channels = Channels::new(bind_tx);

    // Multiple projects will use *.localhost names
    // otherwise we can just run using the standard `localhost`.
    let multiple = result.projects.len() > 1;

    // Collect virual host configurations
    let mut hosts: Vec<HostConfig> = Vec::new();
    result.projects
        .iter()
        .try_for_each(|res| {
            let target = res.state.options.base.clone();
            let redirect_uris = res.state.redirects.collect()?;
            let hostname = res.state.config.get_local_host_name(multiple); 
            let host = HostConfig::new(
                target,
                hostname,
                Some(redirect_uris),
                Some(utils::generate_id(16)));

            // Configire the live reload relay channels
            let (ws_tx, _rx) = broadcast::channel::<Message>(100);
            let reload_tx = ws_tx.clone();

            let host_channel = HostChannel {reload: Some(reload_tx)};
            channels.hosts.entry(host.name.clone()).or_insert(host_channel);

            hosts.push(host);

            Ok::<(), Error>(())
        })?;

    if hosts.is_empty() {
        return Err(Error::NoLiveHosts)
    }

    // Server must have at least a single virtual host
    let host = hosts.swap_remove(0);

    let port = args.get_port();
    let tls = args.tls.clone();
    let mut opts = ServerConfig::new_host(host, port.to_owned(), tls);

    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {

            // Get the socket address and websocket transmission channel
            let info = bind_rx.await.unwrap();
            let url = info.to_url();
            info!("Serve {}", &url);
        });
    });

    // Convert to &'static reference
    let opts = server::configure(opts);

    // Start the webserver
    server::start(opts, &mut channels).await?;

    Ok(())
}
