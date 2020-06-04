use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use std::net::SocketAddrV4;
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

use tokio::sync::broadcast::Sender as TokioSender;
use warp::ws::Message;

use log::{info, debug, error};

use crate::config::Config;
use crate::build::Builder;
use crate::build::loader;
use crate::build::generator;
use crate::command::serve::*;
use crate::{Error};
use crate::utils;

lazy_static! {
    #[derive(Debug)]
    pub static ref ADDR: Arc<Mutex<SocketAddr>> = {
        let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3000);
        Arc::new(Mutex::new(SocketAddr::V4(socket)))
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BuildTag {
    Custom(String),
    Debug,
    Release
}

impl BuildTag {
    pub fn get_path_name(&self) -> String {
        match self {
            BuildTag::Debug => return "debug".to_owned(),
            BuildTag::Release => return "release".to_owned(),
            BuildTag::Custom(s) => return s.to_owned()
        }
    }

    pub fn clone(&self) -> Self {
        match self {
            BuildTag::Debug => return BuildTag::Debug,
            BuildTag::Release => return BuildTag::Release,
            BuildTag::Custom(s) => return BuildTag::Custom(s.to_string())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildOptions {
    pub source: PathBuf,
    pub output: PathBuf,
    pub target: PathBuf,
    pub directory: Option<PathBuf>,
    pub max_depth: Option<usize>,
    pub release: bool,
    pub follow_links: bool,
    pub strict: bool,
    pub clean_url: bool,
    pub tag: BuildTag,
    pub live: bool,
    pub livereload: Option<String>,
    pub host: String,
    pub port: String,
    pub force: bool,
    pub index_links: bool,
}

fn get_websocket_url(options: &BuildOptions, addr: SocketAddr, endpoint: &str) -> String {
    format!("ws://{}:{}/{}", options.host, addr.port(), endpoint)
}

pub fn build(cfg: Config) -> Result<(), Error> {

    let mut options = cfg.build.unwrap();

    if options.live && options.release {
        return Err(
            Error::new("live reload is not available for release builds".to_string()))
    }

    let generators = generator::load(&options)?;

    loader::load(&options)?;

    let mut target = options.source.clone();
    if let Some(dir) = &options.directory {
        target = dir.clone();
    }

    if !options.live {
        let mut builder = Builder::new(&options, &generators);
        builder.load_manifest()?;
        builder.build(&target, false)?;
        return builder.save_manifest()
    } else {
        let endpoint = utils::generate_id(16);

        let opts = ServeOptions {
            target: options.target.clone(),
            watch: Some(target.clone()),
            host: options.host.to_owned(),
            port: options.port.to_owned(),
            endpoint: endpoint.clone(),
            open_browser: false,
        };

        // Create a channel to receive the bind address.
        let (tx, rx) = channel::<(SocketAddr, TokioSender<Message>, String)>();

        // Spawn a thread to receive a notification on the `rx` channel
        // once the server has bound to a port
        std::thread::spawn(move || {
            // Get the socket address and websocket transmission channel
            let (addr, tx, url) = rx.recv().unwrap();

            options.livereload = Some(get_websocket_url(&options, addr, &endpoint));

            // Do a full build before listening for filesystem changes
            let mut serve_builder = Builder::new(&options, &generators);
            if let Err(e) = serve_builder.register_templates_directory() {
                error!("{}", e);
                std::process::exit(1);
            }

            // WARN: must not load_manifest() here otherwise we can have
            // WARN: stale livereload endpoint URLs!

            if let Err(_) = serve_builder.load_manifest() {}

            let result = serve_builder.build(&target, true);

            match result {
                Ok(_) => {

                    // NOTE: only open the browser if initial build succeeds
                    open::that(&url).map(|_| ()).unwrap_or(());

                    #[cfg(feature = "watch")]
                    trigger_on_change(&target.clone(), move |paths, source_dir| {
                        info!("changed({}) in {}", paths.len(), source_dir.display());
                        debug!("files changed: {:?}", paths);
                        if let Ok(invalidation) = serve_builder.get_invalidation(paths) {
                            debug!("invalidation {:?}", invalidation);
                            if let Err(e) = serve_builder.invalidate(&target, invalidation) {
                                error!("{}", e);
                            }
                            serve_builder.save_manifest()?;
                            let _ = tx.send(Message::text("reload"));
                        }

                        Ok(())
                    });
                },
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                }
            }
        });

        // Start the webserver
        serve(opts, tx)?;
    }

    Ok(())
}

