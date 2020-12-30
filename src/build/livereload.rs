use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use log::{debug, error, info};

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use url::Url;
use warp::ws::Message;

use notify::DebouncedEvent::{Create, Remove, Rename, Write};
use notify::RecursiveMode::Recursive;
use notify::Watcher;
use std::thread::sleep;
use std::time::Duration;

use config::server::{ConnectionInfo, HostConfig, PortType, ServerConfig};
use config::ProfileSettings;

use server::{Channels, HostChannel};

use workspace::{Invalidator, Project};

use crate::{Error, ErrorCallback};

struct LiveHost {
    source: PathBuf,
    project: Project,
    websocket: broadcast::Sender<Message>,
}

pub async fn start<P: AsRef<Path>>(
    project: P,
    args: &'static mut ProfileSettings,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    // Prepare the server settings
    let port = args.get_port().clone();
    if port == 0 {
        return Err(Error::NoLiveEphemeralPort);
    }
    let tls = args.tls.clone();

    // Compile the project
    let result = workspace::compile(project, args).await?;

    // Create a channel to receive the bind address.
    let (bind_tx, bind_rx) = oneshot::channel::<ConnectionInfo>();

    // Create the collection of channels
    let mut channels = Channels::new(bind_tx);

    // Multiple projects will use *.localhost names
    // otherwise we can just run using the standard `localhost`.
    let multiple = result.projects.len() > 1;

    let mut watchers: Vec<(LiveHost, mpsc::Receiver<String>)> = Vec::new();

    // Collect virual host configurations
    let mut hosts: Vec<HostConfig> = Vec::new();
    result.projects.into_iter().try_for_each(|project| {
        let target = project.options.base.clone();
        let redirect_uris = project.redirects.collect()?;
        let hostname = project.config.get_local_host_name(multiple);
        let host = HostConfig::new(
            target,
            hostname,
            Some(redirect_uris),
            Some(utils::generate_id(16)),
            false,
        );

        // NOTE: These host names may not resolve so cannot attempt
        // NOTE: to lookup a socket address here.
        let ws_url = config::server::to_websocket_url(
            tls.is_some(),
            &host.name,
            host.endpoint.as_ref().unwrap(),
            config::server::get_port(port.to_owned(), &tls, PortType::Infer),
        );

        // Write out the livereload javascript using the correct
        // websocket endpoint which the server will create later
        livereload::write(&project.config, &host.directory, &ws_url)?;

        // Configure the live reload relay channels
        let (ws_tx, _rx) = broadcast::channel::<Message>(100);
        let reload_tx = ws_tx.clone();

        // Create a channel to receive lazy render requests
        let (render_tx, render_rx) = mpsc::channel::<String>(64);

        let host_channel = HostChannel {
            reload: Some(reload_tx),
            render: Some(render_tx),
        };

        channels
            .hosts
            .entry(host.name.clone())
            .or_insert(host_channel);

        info!("Virtual host: {}", &host.name);

        hosts.push(host);

        // Get the source directory to configure the watcher
        let source = project.options.source.clone();

        watchers.push((
            LiveHost {
                source,
                project,
                websocket: ws_tx,
            },
            render_rx,
        ));

        Ok::<(), Error>(())
    })?;

    if hosts.is_empty() {
        return Err(Error::NoLiveHosts);
    }

    // Server must have at least a single virtual host
    let host = hosts.swap_remove(0);
    let mut opts = ServerConfig::new_host(host, port.to_owned(), tls);
    opts.hosts = hosts;

    // Listen for the bind message and open the browser
    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            // Get the server connection info
            let info = bind_rx.await.unwrap();
            let mut url = info.to_url();

            let path = if let Some(ref path) = args.launch {
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

            url.push_str(&format!("{}?r={}", path, utils::generate_id(4)));

            info!("Serve {}", &url);
            // NOTE: only open the browser if initial build succeeds
            open::that(&url).map(|_| ()).unwrap_or(());
        });
    });

    watch(watchers, error_cb);

    // Convert to &'static reference
    let opts = server::configure(opts);

    // Start the webserver
    server::start(opts, &mut channels).await?;

    Ok(())
}

fn watch(
    watchers: Vec<(LiveHost, mpsc::Receiver<String>)>,
    error_cb: ErrorCallback,
) {
    for (w, mut render) in watchers {
        std::thread::spawn(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                // Create a channel to receive the events.
                let (tx, rx) = std::sync::mpsc::channel();

                let source = w.source.clone();

                // Configure the watcher
                let mut watcher = notify::watcher(tx, Duration::from_secs(1))
                    .expect("Failed to create watcher");

                // NOTE: must start watching in this thread otherwise
                // NOTE: the `rx` channel will be closed prematurely
                watcher
                    .watch(&w.source, Recursive)
                    .expect("Failed to start watcher");
                info!("Watch {}", source.display());

                let invalidator =
                    Arc::new(Mutex::new(Invalidator::new(w.project)));
                let render_invalidator = Arc::clone(&invalidator);
                let ws_tx = &w.websocket;

                // Handle JIT dynamic page renders.
                tokio::task::spawn(async move {
                    while let Some(path) = render.recv().await {
                        let mut live_invalidator =
                            render_invalidator.lock().unwrap();
                        let updater = live_invalidator.updater_mut();
                        let has_page_path = updater.has_page_path(&path);
                        if has_page_path {
                            info!("jit {}", path);
                        }
                    }
                });

                loop {
                    let first_event = rx.recv().unwrap();
                    sleep(Duration::from_millis(50));
                    let other_events = rx.try_iter();
                    let all_events =
                        std::iter::once(first_event).chain(other_events);
                    let paths = all_events
                        .filter_map(|event| {
                            debug!("Received filesystem event: {:?}", event);
                            match event {
                                Create(path)
                                | Write(path)
                                | Remove(path)
                                | Rename(_, path) => Some(path),
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>();

                    if !paths.is_empty() {
                        info!(
                            "Changed({}) in {}",
                            paths.len(),
                            source.display()
                        );

                        let msg = livereload::messages::start();
                        let txt = serde_json::to_string(&msg).unwrap();

                        let _ = ws_tx.send(Message::text(txt));

                        let mut live_invalidator = invalidator.lock().unwrap();

                        match live_invalidator.get_invalidation(paths) {
                            Ok(invalidation) => {
                                // Try to determine a page href to use 
                                // when following edits.
                                let href: Option<String> = if let Some(path) =
                                    invalidation.single_page()
                                {
                                    live_invalidator.find_page_href(path)
                                } else {
                                    None
                                };

                                match live_invalidator
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
            });
        });
    }
}
