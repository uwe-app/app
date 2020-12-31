use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{SystemTime, Duration};
use std::sync::{Arc, RwLock};

use log::{error, info};

use tokio::sync::{
    broadcast,
    mpsc,
    oneshot,
};
use url::Url;
use warp::ws::Message;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use config::server::{
    ConnectionInfo, HostConfig, PortType, ServerConfig, TlsConfig,
};

use workspace::{CompileResult, Invalidator, Project};

use crate::{Result, Error, ErrorCallback, channels::{self, ServerChannels, WatchChannels}};

/// Encpsulates the information needed to watch the 
/// file system and re-render when file changes are detected.
struct LiveHost {
    name: String,
    source: PathBuf,
    project: Project,
}

/// Intermediary value for live projects.
struct LiveResult {
    project: Project,
    source: PathBuf,
    target: PathBuf,
    endpoint: String,
    hostname: String,
}

/// Start watching for file system notifications in the source 
/// directories for the given compiler results.
pub async fn watch(
    port: u16,
    tls: Option<TlsConfig>,
    launch: Option<String>,
    result: CompileResult,
    error_cb: ErrorCallback,
) -> Result<()> {
    // Create a channel to receive the bind address.
    let (bind_tx, bind_rx) = oneshot::channel::<ConnectionInfo>();

    let results = create_resources(port, &tls, result)?;
    let (mut hosts, live_hosts): (Vec<HostConfig>, Vec<LiveHost>) = 
        create_hosts(results)?
        .into_iter()
        .unzip();

    let (server_channels, watch_channels) = create_channels(&live_hosts)?;

    if hosts.is_empty() {
        return Err(Error::NoLiveHosts);
    }

    // Server must have at least a single virtual host
    let host = hosts.swap_remove(0);
    let mut opts = ServerConfig::new_host(host, port.to_owned(), tls);
    opts.hosts = hosts;

    // Spawn the bind listener to launch a browser
    spawn_bind_open(bind_rx, launch);

    // Spawn the file system watchers
    spawn_monitor(live_hosts, Arc::new(RwLock::new(watch_channels)), error_cb);

    // Convert to &'static reference
    let opts = super::configure(opts);

    // Start the webserver
    super::router::serve(opts, bind_tx, Arc::new(RwLock::new(server_channels))).await?;

    Ok(())
}

/// Write out the live reload Javascript and CSS for each 
/// project and create intermediary results.
fn create_resources(
    port: u16,
    tls: &Option<TlsConfig>,
    result: CompileResult) -> Result<Vec<LiveResult>> {

    let mut out: Vec<LiveResult> = Vec::new();
    let mut names: HashMap<String, PathBuf> = HashMap::new();

    // Multiple projects will use *.localhost names
    // otherwise we can just run using the standard `localhost`.
    let multiple = result.projects.len() > 1;

    result.projects.into_iter().try_for_each(|project| {

        let current_project = project.config.project().clone();

        let source = project.options.source.clone();
        let target = project.options.base.clone();

        let hostname = project.config.get_local_host_name(multiple);
        let endpoint = utils::generate_id(16);

        if let Some(ref existing_project) = names.get(&hostname) {
            return Err(Error::DuplicateHostName(
                hostname.clone(),
                existing_project.to_path_buf(),
                current_project));
        }

        names.insert(hostname.clone(), current_project); 

        // NOTE: These host names may not resolve so cannot attempt
        // NOTE: to lookup a socket address here.
        let ws_url = config::server::to_websocket_url(
            tls.is_some(),
            &hostname,
            &endpoint,
            config::server::get_port(port.to_owned(), tls, PortType::Infer),
        );

        // Write out the livereload javascript using the correct
        // websocket endpoint which the server will create later
        livereload::write(&project.config, &target, &ws_url)?;

        out.push(LiveResult {project, source, target, endpoint, hostname});

        Ok::<(), Error>(())
    })?;

    Ok(out)
}

/// Create host configurations paired with live host configurations which 
/// contain data for file system watching and the channels used for message 
/// passing.
fn create_hosts(results: Vec<LiveResult>) -> Result<Vec<(HostConfig, LiveHost)>> {
    let mut out: Vec<(HostConfig, LiveHost)> = Vec::new();

    results.into_iter().try_for_each(|result| {
        let project = result.project;
        let source = result.source;
        let target = result.target;
        let hostname = result.hostname;
        let endpoint = result.endpoint;

        // TODO: fix redirect URIs
        let redirect_uris = project.redirects.collect()?;

        info!("Virtual host: {}", &hostname);

        let host = HostConfig::new(
            target,
            hostname,
            Some(redirect_uris),
            Some(endpoint),
            false,
        );

        let live_host = LiveHost {
            name: host.name.clone(),
            source,
            project,
        };

        out.push((host, live_host));

        Ok::<(), Error>(())
    })?;
    Ok(out)
}

fn create_channels(results: &Vec<LiveHost>) -> Result<(ServerChannels, WatchChannels)> {

    // Create the collection of channels
    let mut server: ServerChannels = Default::default();
    let mut watch: WatchChannels = Default::default();

    results.iter().try_for_each(|live_host| {

        // Configure the live reload relay channels
        let (ws_tx, _ws_rx) = broadcast::channel::<Message>(128);
        server.websockets.insert(live_host.name.clone(), ws_tx.clone());
        watch.websockets.insert(live_host.name.clone(), ws_tx);

        // Create a channel to receive lazy render requests
        let (request_tx, request_rx) = mpsc::channel::<String>(channels::RENDER_CHANNEL_BUFFER);
        server.render.insert(live_host.name.clone(), request_tx);
        watch.render.insert(live_host.name.clone(), request_rx);

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
    launch: Option<String>) {

    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
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
    watchers: Vec<LiveHost>,
    channels: Arc<RwLock<WatchChannels>>,
    error_cb: ErrorCallback,
) {
    for w in watchers {
        let watch_channels = Arc::clone(&channels);

        std::thread::spawn(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap();
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
                let request = channels_access.render.get_mut(&name).unwrap();

                loop {
                    tokio::select! {
                        val = request.recv() => {
                            //let reponse_tx = channels.render_responses.get(&w.name).clone().unwrap();

                            if let Some(path) = val {
                                //println!("Got web server render request: {}", &path);
                                let updater = invalidator.updater_mut();
                                let has_page_path = updater.has_page_path(&path);
                                if has_page_path {
                                    info!("JIT {}", &path);
                                    match updater.render(&path).await {
                                        Ok(_) => {},
                                        Err(e) => {
                                            // TODO: send error back to the server for a 500 response?
                                            error!("{}", e);
                                            //response.
                                        }
                                    }
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
                                    if let Ok(event) = fs_rx.try_recv() {
                                        event_buffer.push(event);
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
