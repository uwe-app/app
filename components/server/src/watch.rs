use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use log::{error, info};

use futures_util::FutureExt;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{broadcast, mpsc, oneshot::{self, error::TryRecvError}, Mutex};
use url::Url;
use psup_impl::Worker;
use psup_json_rpc::{write, notify};

use config::server::{
    ConnectionInfo, HostConfig, ServerConfig, SslConfig,
};

use workspace::{CompileResult, HostInfo, HostResult, Invalidator};

use crate::{
    channels::{self, Message, ResponseValue, ServerChannels, WatchChannels},
    Error, ErrorCallback, Result, ServerSettings,
};

/// Start watching for file system notifications in the source
/// directories for the given compiler results.
pub async fn watch(
    listen: Option<String>,
    port: u16,
    tls: Option<SslConfig>,
    launch: Option<String>,
    headless: bool,
    result: CompileResult,
    authorities: Option<Vec<String>>,
    error_cb: ErrorCallback,
) -> Result<()> {
    // Create a channel to receive the bind address.
    let (bind_tx, bind_rx) = oneshot::channel::<ConnectionInfo>();
    let (_shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();

    let host_result: HostResult = result.into();
    let mut host_configs: Vec<(HostInfo, HostConfig)> =
        host_result.try_into()?;

    for (_info, host) in host_configs.iter_mut() {
        host.set_watch(true);
        host.set_disable_cache(true);
    }

    let (host_info, hosts): (Vec<HostInfo>, Vec<HostConfig>) =
        host_configs.into_iter().unzip();

    if hosts.is_empty() {
        return Err(Error::NoLiveHosts);
    }

    create_resources(&host_info)?;

    let channel_names = hosts
        .iter()
        .map(|h| h.name().to_string())
        .collect::<Vec<String>>();

    let (server_channels, watch_channels) = create_channels(channel_names)?;

    // Server must have at least a single virtual host
    let mut opts = ServerConfig::new(
        listen.unwrap_or(config::ADDR.to_string()),
        port.to_owned(),
        tls,
    );


    opts.set_authorities(authorities);
    opts.set_hosts(hosts);
    opts.set_disable_signals(true);

    let (worker_tx, worker_rx) = oneshot::channel::<ConnectionInfo>();
    let connection_rx = Arc::new(Mutex::new(worker_rx));

    // Set up a worker to pass on connection info to a supervisor process.
    //
    // This should only be required when the UI editor is running which needs
    // to supervise the child processes per project.
    tokio::task::spawn(async move {
        let worker = Worker::new()
            .client(|stream, id| async {
                let (_reader, mut writer) = stream.into_split();
                let mut rx = connection_rx.lock().await;
                loop {
                    match rx.try_recv() {
                        Ok(info) => {
                            let bridge = project::ConnectionBridge::new(id, info);
                            let params =
                                serde_json::to_value(&bridge).map_err(Box::from)?;
                            let req = notify("connected", Some(params));
                            write(&mut writer, &req).await?;
                            break;
                        }
                        Err(TryRecvError::Closed) => break,
                        _ => {}
                    }
                }
                Ok::<(), psup_impl::Error>(())
            })
            .relaxed(true);

        worker.run().await?;
        Ok::<(), psup_impl::Error>(())
    });

    // Spawn the bind listener to launch a browser
    spawn_bind_open(bind_rx, worker_tx, launch, headless);

    let number_watchers = host_info.len();
    let mut watchers_started = 0usize;
    let (watcher_tx, mut watcher_rx) = mpsc::channel::<bool>(number_watchers);

    // Spawn the file system watchers
    spawn_monitor(
        host_info,
        Arc::new(RwLock::new(watch_channels)),
        watcher_tx,
        error_cb,
    );

    // Must wait for all the watchers to set up channels before starting the web server
    while let Some(_) = watcher_rx.recv().await {
        watchers_started += 1;
        if watchers_started == number_watchers {
            break;
        }
    }

    // Start the webserver
    super::router::serve(ServerSettings {
        config: opts,
        bind: bind_tx,
        shutdown: shutdown_rx,
        channels: server_channels,
    })
    .await?;

    Ok(())
}

/// Write out the live reload Javascript and CSS.
fn create_resources(
    hosts: &Vec<HostInfo>,
) -> Result<()> {
    hosts.iter().try_for_each(|host| {
        // Write out the livereload javascript using the correct
        // websocket endpoint which the server will create later
        livereload::write(&host.project.config, &host.target, &host.endpoint)?;
        Ok::<(), Error>(())
    })?;

    Ok(())
}

fn create_channels(
    names: Vec<String>,
) -> Result<(ServerChannels, WatchChannels)> {
    // Create the collection of channels
    let mut server = ServerChannels::new();
    let mut watch: WatchChannels = Default::default();

    names.iter().try_for_each(|name| {
        // Configure the live reload relay channels
        let (ws_tx, _ws_rx) = broadcast::channel::<Message>(128);
        server.websockets.insert(name.clone(), ws_tx.clone());
        watch.websockets.insert(name.clone(), ws_tx);

        // Create a channel to receive lazy render requests
        let (request_tx, request_rx) =
            mpsc::channel::<(String, oneshot::Sender<ResponseValue>)>(
                channels::RENDER_CHANNEL_BUFFER,
            );
        server.render.insert(name.clone(), request_tx);
        watch.render.insert(name.clone(), request_rx);

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
    bind_tx: oneshot::Sender<ConnectionInfo>,
    launch: Option<String>,
    headless: bool,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            // Get the server connection info so we
            // can open a browser with the correct URL
            match bind_rx.await {
                Ok(info) => {
                    let _ = bind_tx.send(info.clone());

                    let mut url = info.to_url();

                    if !headless {
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
                        url.push_str(&format!(
                            "{}?r={}",
                            path,
                            utils::generate_id(4)
                        ));

                        info!("Serve {}", &url);

                        open::that(&url).map(|_| ()).unwrap_or(());
                    }

                }
                _ => {}
            }
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
                //let response = channels_access.render_responses.get(&name).unwrap().clone();

                // NOTE: must `remove` the receiver and drop `channels_access` so that
                // NOTE: multiple virtual hosts start up as expected
                let mut request = channels_access.render.remove(&name).unwrap();
                drop(channels_access);

                // Notify that this watcher is ready to accept messages
                let _ = started_tx.send(true).await;

                loop {
                    tokio::select! {
                        val = request.recv() => {
                            if let Some((path, resp_tx)) = val {
                                let updater = invalidator.updater_mut();
                                let has_page_path = updater.has_page_path(&path);
                                if has_page_path {
                                    info!("SSR {}", &path);
                                    match updater.render(&path).await {
                                        Ok(_) => {
                                            let _ = resp_tx.send(None);
                                        },
                                        Err(e) => {
                                            // Send error back to the server so it can
                                            // show a 500 error if the compile fails
                                            error!("{}", e);
                                            let _ = resp_tx.send(Some(Box::new(e)));
                                        }
                                    }
                                } else {
                                    // Must always send a reply as the web server
                                    // blocks waiting for one
                                    let _ = resp_tx.send(None);
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

                                    let _ = ws_tx.send(Message::Text(txt));

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
                                                    let _ = ws_tx.send(Message::Text(txt));
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
                                                    let _ = ws_tx.send(Message::Text(txt));
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
