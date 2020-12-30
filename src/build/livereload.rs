use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::time::SystemTime;

use log::{error, info};

use tokio::sync::{broadcast, mpsc::{self, UnboundedSender, UnboundedReceiver}, oneshot};
use url::Url;
use warp::ws::Message;

use notify::{Watcher, RecommendedWatcher, RecursiveMode};

use std::time::Duration;

use config::server::{ConnectionInfo, HostConfig, PortType, ServerConfig};
use config::ProfileSettings;

use server::{Channels, HostChannel};

use workspace::{Invalidator, Project};

use crate::{Error, ErrorCallback};

type RenderResponse = Option<Box<dyn std::error::Error + Send>>;

struct LiveHost {
    name: String,
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

    let mut watchers: Vec<(LiveHost, UnboundedReceiver<String>)> = Vec::new();

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
        let (request_tx, request_rx) = mpsc::unbounded_channel::<String>();

        let host_channel = HostChannel::new(reload_tx, request_tx);
        let name = host.name.clone();

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
                name,
                source,
                project,
                websocket: ws_tx,
            },
            request_rx,
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
    watchers: Vec<(LiveHost, mpsc::UnboundedReceiver<String>)>,
    error_cb: ErrorCallback,
    //channels: &Channels,
) {
    for (w, mut request) in watchers {

        std::thread::spawn(move || {
            // NOTE: We want to schedule all async task on the same thread!
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

                info!("Watch {}", source.display());

                let mut invalidator = Invalidator::new(w.project);

                let ws_tx = &w.websocket;

                loop {
                    tokio::select! {
                        val = request.recv() => {
                            //let reponse_tx = channels.render_responses.get(&w.name).clone().unwrap();

                            if let Some(path) = val {
                                println!("Got web server render request: {}", &path);
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

                //loop {
                    //let first_event = rx.recv().unwrap();
                    //sleep(Duration::from_millis(50));
                    //let other_events = rx.try_iter();
                    //let all_events =
                        //std::iter::once(first_event).chain(other_events);

                    /*
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
                */
                //}
            });
        });
    }
}
