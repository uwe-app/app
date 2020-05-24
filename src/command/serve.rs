// Code derived from: https://github.com/rust-lang/mdBook/blob/master/src/cmd/serve.rs
// Respect to the original authors.
//
// Modified to gracefully handle ephemeral port.

#[cfg(feature = "watch")]
use crate::{open};
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::sync::broadcast;
use warp::ws::Message;
use warp::Filter;
use std::convert::AsRef;

use notify::Watcher;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;

use crate::{Error, LIVE_RELOAD_ENDPOINT};
use log::{info, trace, error, debug};

#[derive(Debug)]
pub struct ServeOptions {
    pub target: PathBuf,
    pub host: String,
    pub port: String,
    pub open_browser: bool,
    pub watch: Option<PathBuf>,
}

impl ServeOptions {
    pub fn new(target: PathBuf, watch: PathBuf, host: String, port: String) -> Self {
        ServeOptions {
            target,
            watch: Some(watch),
            host,
            port,
            open_browser: true,
        } 
    }
}

pub fn serve<F>(options: ServeOptions, mut callback: F) -> Result<(), Error>
    where
        F: FnMut(Vec<PathBuf>, &Path) -> Result<(), Error>,
    {

    let address = format!("{}:{}", options.host, options.port);
    let sockaddr: SocketAddr = address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| Error::new(format!("no address found for {}", address)))?;

    let build_dir = options.target.clone();
    let host = options.host.clone();
    let open_browser = options.open_browser.clone();

    // A channel used to broadcast to any websockets to reload when a file changes.
    let (tx, _rx) = tokio::sync::broadcast::channel::<Message>(100);

    let reload_tx = tx.clone();
    let thread_handle = std::thread::spawn(move || {
        serve_web(
            build_dir,
            &host,
            open_browser,
            sockaddr,
            reload_tx);
    });

    if let Some(p) = options.watch {
        let source_dir = p.as_path();
        #[cfg(feature = "watch")]
        trigger_on_change(source_dir, move |paths, source_dir| {
            if let Ok(_) = callback(paths, source_dir) {
                let _ = tx.send(Message::text("reload"));
            }
        });
    }

    let _ = thread_handle.join();

    Ok(())
}

pub fn trigger_on_change<P, F>(dir: P, mut closure: F)
where
    P: AsRef<Path>,
    F: FnMut(Vec<PathBuf>, &Path),
{
    use notify::DebouncedEvent::*;
    use notify::RecursiveMode::*;

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    let mut watcher = match notify::watcher(tx, Duration::from_secs(1)) {
        Ok(w) => w,
        Err(e) => {
            error!("Error while trying to watch the files:\n\n\t{:?}", e);
            std::process::exit(1)
        }
    };

    // FIXME: if --directory we must also watch data.toml and layout.hbs

    // Add the source directory to the watcher
    if let Err(e) = watcher.watch(&dir, Recursive) {
        error!("Error while watching {:?}:\n    {:?}", dir.as_ref().display(), e);
        std::process::exit(1);
    };

    //let _ = watcher.watch(book.theme_dir(), Recursive);
    // Add the book.toml file to the watcher if it exists
    //let _ = watcher.watch(book.root.join("book.toml"), NonRecursive);

    info!("watch {}", dir.as_ref().display());

    loop {
        let first_event = rx.recv().unwrap();
        sleep(Duration::from_millis(50));
        let other_events = rx.try_iter();

        let all_events = std::iter::once(first_event).chain(other_events);

        let paths = all_events
            .filter_map(|event| {
                debug!("Received filesystem event: {:?}", event);

                match event {
                    Create(path) | Write(path) | Remove(path) | Rename(_, path) => Some(path),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        if !paths.is_empty() {
            closure(paths, &dir.as_ref());
        }
    }
}

#[tokio::main]
async fn serve_web(
    build_dir: PathBuf,
    host: &str,
    open_browser: bool,
    address: SocketAddr,
    reload_tx: broadcast::Sender<Message>) {

    // A warp Filter which captures `reload_tx` and provides an `rx` copy to
    // receive reload messages.
    let sender = warp::any().map(move || reload_tx.subscribe());

    // A warp Filter to handle the livereload endpoint. This upgrades to a
    // websocket, and then waits for any filesystem change notifications, and
    // relays them over the websocket.
    let livereload = warp::path(LIVE_RELOAD_ENDPOINT)
        .and(warp::ws())
        .and(sender)
        .map(|ws: warp::ws::Ws, mut rx: broadcast::Receiver<Message>| {
            ws.on_upgrade(move |ws| async move {
                let (mut user_ws_tx, _user_ws_rx) = ws.split();
                trace!("websocket got connection");
                if let Ok(m) = rx.recv().await {
                    trace!("notify of reload");
                    let _ = user_ws_tx.send(m).await;
                }
            })
        });

    // A warp Filter that serves from the filesystem.
    let static_route = warp::fs::dir(build_dir);
    let routes = livereload.or(static_route);

    //println!("bind socket address is {:?}", address.port());

    let bind_result = warp::serve(routes).try_bind_ephemeral(address);
    match bind_result {
        Ok((addr, future)) => {
            let serving_url = format!("http://{}:{}", host, addr.port());
            info!("serve {}", serving_url);
            if open_browser {
                // It is ok if this errors we just don't open a browser window
                open::that(serving_url).map(|_| ()).unwrap_or(());
            }
            future.await;
        },
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}
