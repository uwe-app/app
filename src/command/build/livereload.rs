use std::path::{Path, PathBuf};
use std::sync::mpsc;

use log::{debug, error, info};

use tokio::sync::broadcast;
use tokio::sync::oneshot;
use warp::ws::Message;

use notify::DebouncedEvent::{self, Create, Remove, Rename, Write};
use notify::RecursiveMode::Recursive;
use notify::Watcher;
use std::thread::sleep;
use std::time::Duration;

use config::server::{ConnectionInfo, HostConfig, PortType, ServerConfig};
use config::ProfileSettings;

use server::{Channels, HostChannel};

use workspace::{Invalidator, Render};

use crate::{Error, ErrorCallback};

struct LiveHost<'s> {
    source: PathBuf,
    receiver: mpsc::Receiver<DebouncedEvent>,
    sender: mpsc::Sender<DebouncedEvent>,
    state: Render<'s>,
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

    let mut watchers: Vec<LiveHost<'_>> = Vec::new();

    // Collect virual host configurations
    let mut hosts: Vec<HostConfig> = Vec::new();
    result.projects.into_iter().try_for_each(|state| {
        let target = state.options.base.clone();
        let redirect_uris = state.redirects.collect()?;
        let hostname = state.config.get_local_host_name(multiple);
        let host = HostConfig::new(
            target,
            hostname,
            Some(redirect_uris),
            Some(utils::generate_id(16)),
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
        livereload::write(&state.config, &host.directory, &ws_url)?;

        // Configure the live reload relay channels
        let (ws_tx, _rx) = broadcast::channel::<Message>(100);
        let reload_tx = ws_tx.clone();

        let host_channel = HostChannel {
            reload: Some(reload_tx),
        };
        channels
            .hosts
            .entry(host.name.clone())
            .or_insert(host_channel);

        info!("Virtual host: {}", &host.name);

        hosts.push(host);

        // Get the source directory to configure the watcher
        let source = state.options.source.clone();
        // Create a channel to receive the events.
        let (tx, rx) = mpsc::channel();

        watchers.push(LiveHost {
            source,
            state,
            websocket: ws_tx,
            receiver: rx,
            sender: tx,
        });

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
            let url = info.to_url();
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

fn watch(watchers: Vec<LiveHost<'static>>, error_cb: ErrorCallback) {
    for mut w in watchers {
        std::thread::spawn(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let rx = w.receiver;
                let tx = w.sender;

                // Configure the watcher
                let mut watcher = notify::watcher(tx, Duration::from_secs(1))
                    .expect("Failed to create watcher");

                // NOTE: must start watching in this thread otherwise
                // NOTE: the `rx` channel will be closed prematurely
                watcher
                    .watch(&w.source, Recursive)
                    .expect("Failed to start watcher");
                info!("Watch {}", w.source.display());

                let mut invalidator = Invalidator::new(&mut w.state);
                let ws_tx = &w.websocket;

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
                            w.source.display()
                        );

                        let msg = livereload::messages::start();
                        let txt = serde_json::to_string(&msg).unwrap();

                        let _ = ws_tx.send(Message::text(txt));

                        let result = invalidator.get_invalidation(paths);
                        match result {
                            Ok(invalidation) => {
                                if let Err(e) = invalidator
                                    .invalidate(&w.source, &invalidation)
                                    .await
                                {
                                    error!("{}", e);

                                    let msg = livereload::messages::notify(
                                        e.to_string(),
                                        true,
                                    );
                                    let txt =
                                        serde_json::to_string(&msg).unwrap();
                                    let _ = ws_tx.send(Message::text(txt));

                                //return error_cb(Error::from(e));
                                } else {
                                    //self.builder.manifest.save()?;
                                    if invalidation.notify {
                                        let msg =
                                            livereload::messages::reload();
                                        let txt = serde_json::to_string(&msg)
                                            .unwrap();
                                        let _ = ws_tx.send(Message::text(txt));
                                        //println!("Got result {:?}", res);
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
