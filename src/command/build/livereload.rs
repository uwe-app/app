use std::net::SocketAddr;
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
//use compiler::invalidator::Invalidator;
use compiler::parser::Parser;
use compiler::redirect;
use config::ProfileSettings;

use crate::command::run::{self, ServeOptions};
use crate::{Error, ErrorCallback};

use super::invalidator::Invalidator;

fn get_websocket_url(host: String, addr: SocketAddr, endpoint: &str) -> String {
    format!("ws://{}:{}/{}", host, addr.port(), endpoint)
}

pub async fn start<P: AsRef<Path>>(
    project: P,
    args: &mut ProfileSettings,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let (ctx, locales) = workspace::compile_project(project, args).await?;

    let host = ctx.options.settings.get_host();
    let port = ctx.options.settings.get_port();

    let source = ctx.options.source.clone();
    let endpoint = utils::generate_id(16);

    let mut redirect_uris = None;

    if let Some(ref redirects) = ctx.config.redirect {
        redirect_uris = Some(redirect::collect(redirects)?);
    }

    let target = ctx.options.base.clone().to_path_buf();

    let opts = ServeOptions {
        target,
        watch: Some(source.clone()),
        host: host.to_owned(),
        port: port.to_owned(),
        endpoint: endpoint.clone(),
        open_browser: false,
        redirects: redirect_uris,
    };

    // Create a channel to receive the bind address.
    let (bind_tx, bind_rx) = oneshot::channel::<(SocketAddr, String)>();

    let (ws_tx, _rx) = broadcast::channel::<Message>(100);
    let reload_tx = ws_tx.clone();

    // Spawn a thread to receive a notification on the `rx` channel
    // once the server has bound to a port
    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            // Get the socket address and websocket transmission channel
            let (addr, url) = bind_rx.await.unwrap();

            let ws_url = get_websocket_url(host, addr, &endpoint);

            if let Err(e) = livereload::write(&ctx.config, &ctx.options.target, &ws_url) {
                return error_cb(Error::from(e));
            }

            // Must be in a new scope so the write lock is dropped
            // before compilation and invalidation
            {
                let mut livereload = compiler::context::livereload().write().unwrap();
                *livereload = Some(ws_url);
            }

            let parser = Parser::new(&ctx, &locales).unwrap();
            let compiler = Compiler::new(&ctx);

            // NOTE: only open the browser if initial build succeeds
            open::that(&url).map(|_| ()).unwrap_or(());

            // Invalidator wraps the builder receiving filesystem change
            // notifications and sending messages over the `tx` channel
            // to connected websockets when necessary
            //
            let mut invalidator = Invalidator::new(compiler, parser);

            // Create a channel to receive the events.
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match notify::watcher(tx, Duration::from_secs(1)) {
                Ok(w) => w,
                Err(e) => return error_cb(Error::from(e)),
            };

            // Add the source directory to the watcher
            if let Err(e) = watcher.watch(&source, Recursive) {
                return error_cb(Error::from(e));
            };

            info!("Watch {}", source.display());

            loop {
                let first_event = rx.recv().unwrap();
                sleep(Duration::from_millis(50));
                let other_events = rx.try_iter();
                let all_events = std::iter::once(first_event).chain(other_events);
                let paths = all_events
                    .filter_map(|event| {
                        debug!("Received filesystem event: {:?}", event);
                        match event {
                            Create(path) | Write(path) | Remove(path) | Rename(_, path) => {
                                Some(path)
                            }
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>();

                if !paths.is_empty() {
                    info!("Changed({}) in {}", paths.len(), source.display());

                    let msg = livereload::messages::start();
                    let txt = serde_json::to_string(&msg).unwrap();

                    let _ = ws_tx.send(Message::text(txt));

                    let result = invalidator.get_invalidation(paths);
                    match result {
                        Ok(invalidation) => {
                            if let Err(e) = invalidator.invalidate(&source, &invalidation).await {
                                error!("{}", e);

                                let msg = livereload::messages::notify(e.to_string(), true);
                                let txt = serde_json::to_string(&msg).unwrap();
                                let _ = ws_tx.send(Message::text(txt));

                            //return error_cb(Error::from(e));
                            } else {
                                //self.builder.manifest.save()?;
                                if invalidation.notify {
                                    let msg = livereload::messages::reload();
                                    let txt = serde_json::to_string(&msg).unwrap();
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

    // Start the webserver
    run::serve(opts, reload_tx, bind_tx).await?;

    Ok(())
}
