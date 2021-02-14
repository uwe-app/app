use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use log::{error, info};

use futures_util::FutureExt;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{broadcast, mpsc, oneshot};
use url::Url;
use warp::ws::Message;

use config::server::{
    ConnectionInfo, HostConfig, PortType, ServerConfig, TlsConfig, WebDavConfig,
};

use workspace::{CompileResult, HostInfo, HostResult, Invalidator};

use crate::{
    channels::{self, ResponseValue, ServerChannels, WatchChannels},
    Error, ErrorCallback, Result,
};

/// Start watching for file system notifications in the source
/// directories for the given compiler results.
pub async fn watch(
    port: u16,
    tls: Option<TlsConfig>,
    launch: Option<String>,
    headless: bool,
    result: CompileResult,
    webdav_enabled: bool,
    bind_host: Option<String>,
    authorities: Option<Vec<String>>,
    error_cb: ErrorCallback,
) -> Result<()> {
    // Create a channel to receive the bind address.
    let (bind_tx, bind_rx) = oneshot::channel::<ConnectionInfo>();

    let host_result: HostResult = result.into();
    let mut host_configs: Vec<(HostInfo, HostConfig)> = host_result.try_into()?;

    for (info, host) in host_configs.iter_mut() {
        host.watch = true;
        if webdav_enabled {
            host.webdav = Some(WebDavConfig {
                directory: info.source.to_path_buf(),
                listing: false,
            });
        }
    }

    let (host_info, mut hosts): (Vec<HostInfo>, Vec<HostConfig>) =
        host_configs.into_iter().unzip();

    create_resources(port, &tls, &host_info)?;

    let (server_channels, watch_channels) = create_channels(&host_info)?;

    if hosts.is_empty() {
        return Err(Error::NoLiveHosts);
    }

    // Server must have at least a single virtual host
    let host = hosts.swap_remove(0);
    let mut opts = ServerConfig::new_host(host, port.to_owned(), tls);
    opts.authorities = authorities;
    opts.hosts = hosts;

    if let Some(ref host) = bind_host {
        opts.listen = host.to_string();
    }

    // Spawn the bind listener to launch a browser
    if !headless {
        spawn_bind_open(bind_rx, launch);
    }

    let number_watchers = host_info.len();
    let mut watchers_started = 0usize;
    let (watcher_tx, mut watcher_rx) = mpsc::channel::<bool>(number_watchers);

    // Spawn the file system watchers
    spawn_monitor(host_info, Arc::new(RwLock::new(watch_channels)), watcher_tx, error_cb);

    // Must wait for all the watchers to set up channels before starting the web server
    while let Some(_) = watcher_rx.recv().await {
        watchers_started += 1;
        if watchers_started == number_watchers {
            break;
        }
    }

    // Convert to &'static reference
    let opts = super::configure(opts);

    // Start the webserver
    super::router::serve(opts, bind_tx, server_channels).await?;

    Ok(())
}

/// Write out the live reload Javascript and CSS.
fn create_resources(
    port: u16,
    tls: &Option<TlsConfig>,
    hosts: &Vec<HostInfo>,
) -> Result<()> {
    hosts.iter().try_for_each(|host| {
        // NOTE: These host names may not resolve so cannot attempt
        // NOTE: to lookup a socket address here.
        let ws_url = config::server::to_websocket_url(
            tls.is_some(),
            &host.name,
            &host.endpoint,
            config::server::get_port(port.to_owned(), tls, PortType::Infer),
        );

        // Write out the livereload javascript using the correct
        // websocket endpoint which the server will create later
        livereload::write(&host.project.config, &host.target, &ws_url)?;

        Ok::<(), Error>(())
    })?;

    Ok(())
}

fn create_channels(
    results: &Vec<HostInfo>,
) -> Result<(ServerChannels, WatchChannels)> {
    // Create the collection of channels

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let mut server = ServerChannels::new(shutdown_tx, shutdown_rx);
    let mut watch: WatchChannels = Default::default();

    results.iter().try_for_each(|host| {
        // Configure the live reload relay channels
        let (ws_tx, _ws_rx) = broadcast::channel::<Message>(128);
        server.websockets.insert(host.name.clone(), ws_tx.clone());
        watch.websockets.insert(host.name.clone(), ws_tx);

        // Create a channel to receive lazy render requests
        let (request_tx, request_rx) =
            mpsc::channel::<String>(channels::RENDER_CHANNEL_BUFFER);
        server.render.insert(host.name.clone(), request_tx);
        watch.render.insert(host.name.clone(), request_rx);

        // Create a channel for replies when rendering
        let (response_tx, response_rx) =
            mpsc::channel::<ResponseValue>(channels::RENDER_CHANNEL_BUFFER);
        server
            .render_responses
            .insert(host.name.clone(), response_rx);
        watch
            .render_responses
            .insert(host.name.clone(), response_tx);

        Ok::<(), Error>(())
    })?;

    Ok((server, watch))
}

/// Spawn a thread that listens for the bind message from the
/// server and opens the browser once the message is received.
///
/// By listening for the bind message the browser is not launched
/// the browser is not opened when a server error such as EADDR is
/// encountered.
fn spawn_bind_open(
    bind_rx: oneshot::Receiver<ConnectionInfo>,
    launch: Option<String>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            // Get the server connection info
            let info = bind_rx.await.unwrap();
            let mut url = info.to_url();

            let path = if let Some(ref path) = launch {
                // If we get an absolute URL just use the path
                let url_path = if let Ok(url) = path.parse::<Url>() {
                    url.path().to_string()
                } else {
                    path.to_string()
                };

                // Allow for path strings to omit the leading slash
                let url_path = url_path.trim_start_matches("/");
                format!("/{}", url_path)
            } else {
                "/".to_string()
            };

            // Ensure the cache is bypassed so that switching between
            // projects does not show an older project
            url.push_str(&format!("{}?r={}", path, utils::generate_id(4)));

            info!("Serve {}", &url);
            open::that(&url).map(|_| ()).unwrap_or(());
        });
    });
}

/// Spawn a thread for each virtual host that requires a
/// file system watcher.
fn spawn_monitor(
    watchers: Vec<HostInfo>,
    channels: Arc<RwLock<WatchChannels>>,
    watcher_tx: mpsc::Sender<bool>,
    error_cb: ErrorCallback,
) {
    for w in watchers {
        let watch_channels = Arc::clone(&channels);

        let started_tx = watcher_tx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                // Wrap the fs channel so we can select on the future
                let (fs_tx, mut fs_rx) = mpsc::unbounded_channel();

                let name = w.name.clone();
                let source = w.source.clone();

                let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| {
                    let tx = fs_tx.clone();
                    match res {
                        Ok(event) => {
                            tx.send(event).expect("Failed to send file system event");
                        },
                        Err(e) => error!("Watch error: {:?}", e),
                    }
                }).expect("Failed to create watcher");

                // Add a path to be watched. All files and directories at that path and
                // below will be monitored for changes.
                watcher.watch(&w.source, RecursiveMode::Recursive)
                    .expect("Failed to start watching");

                info!("Watch {} in {}", name, source.display());

                let mut invalidator = Invalidator::new(w.project);
                let mut channels_access = watch_channels.write().unwrap();
                let ws_tx = channels_access.websockets.get(&name).unwrap().clone();
                let response = channels_access.render_responses.get(&name).unwrap().clone();

                // NOTE: must `remove` the receiver and drop `channels_access` so that
                // NOTE: multiple virtual hosts start up as expected
                let mut request = channels_access.render.remove(&name).unwrap();
                drop(channels_access);

                // Notify that this watcher is ready to accept messages
                let _ = started_tx.send(true).await;

                loop {
                    tokio::select! {
                        val = request.recv() => {
                            if let Some(path) = val {
                                let updater = invalidator.updater_mut();
                                let has_page_path = updater.has_page_path(&path);
                                if has_page_path {
                                    info!("SSR {}", &path);
                                    match updater.render(&path).await {
                                        Ok(_) => {
                                            let _ = response.send(None).await;
                                        },
                                        Err(e) => {
                                            // Send error back to the server so it can
                                            // show a 500 error if the compile fails
                                            error!("{}", e);
                                            let _ = response.send(Some(Box::new(e))).await;
                                        }
                                    }
                                } else {
                                    // Must always send a reply as the web server
                                    // blocks waiting for one
                                    let _ = response.send(None).await;
                                }
                            }
                        }
                        val = fs_rx.recv() => {
                            if let Some(event) = val {
                                // Buffer because multiple events for the same
                                // file can fire in rapid succesion
                                let mut event_buffer = vec![event];
                                let start = SystemTime::now();
                                while SystemTime::now().duration_since(start).unwrap() < Duration::from_millis(50) {
                                    // NOTE: Used to use try_recv() but it was removed in tokio@1.0
                                    // SEE: https://github.com/tokio-rs/tokio/releases/tag/tokio-1.0.0
                                    // SEE: https://github.com/tokio-rs/tokio/pull/3263
                                    // SEE: https://github.com/tokio-rs/tokio/issues/3350
                                    if let Some(event) = fs_rx.recv().now_or_never() {
                                        if let Some(ev) = event {
                                            event_buffer.push(ev);
                                        }
                                    }
                                }

                                let paths = event_buffer
                                    .iter()
                                    .map(|event| {
                                        event.paths.clone()
                                    })
                                    .flatten()
                                    .collect::<HashSet<_>>();

                                if !paths.is_empty() {
                                    info!(
                                        "Changed({}) in {}",
                                        paths.len(),
                                        source.display()
                                    );

                                    let msg = livereload::messages::start();
                                    let txt = serde_json::to_string(&msg).unwrap();

                                    let _ = ws_tx.send(Message::text(txt));

                                    match invalidator.get_invalidation(paths) {
                                        Ok(invalidation) => {

                                            // Try to determine a page href to use
                                            // when following edits.
                                            let href: Option<String> = if let Some(path) =
                                                invalidation.single_page()
                                            {
                                                invalidator.find_page_href(path)
                                            } else {
                                                None
                                            };

                                            match invalidator
                                                .updater_mut()
                                                .invalidate(&invalidation)
                                                .await
                                            {
                                                // Notify of build completed
                                                Ok(_) => {
                                                    let msg =
                                                        livereload::messages::reload(href);
                                                    let txt = serde_json::to_string(&msg)
                                                        .unwrap();
                                                    let _ = ws_tx.send(Message::text(txt));
                                                    //println!("Got result {:?}", res);
                                                }
                                                // Send errors to the websocket
                                                Err(e) => {
                                                    error!("{}", e);

                                                    let msg = livereload::messages::notify(
                                                        e.to_string(),
                                                        true,
                                                    );
                                                    let txt = serde_json::to_string(&msg)
                                                        .unwrap();
                                                    let _ = ws_tx.send(Message::text(txt));
                                                }
                                            }
                                        }
                                        Err(e) => return error_cb(Error::from(e)),
                                    }


                                }

                            }
                        }
                    }
                }
            });
        });
    }
}
