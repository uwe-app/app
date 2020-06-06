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
use crate::build::generator::GeneratorMap;
use crate::build::loader;
use crate::build::context;
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
    // Root of the input
    pub source: PathBuf,
    // Root of the output
    pub output: PathBuf,
    // Target output directory including a build tag
    pub target: PathBuf,
    // Specific directory relative to source to walk
    pub directory: Option<PathBuf>,
    // Where to build from either `source` or `directory` relative to `source`
    pub from: PathBuf,

    pub max_depth: Option<usize>,

    pub release: bool,
    pub clean_url: bool,
    pub tag: BuildTag,
    pub live: bool,
    pub host: String,
    pub port: u16,
    pub force: bool,
    pub index_links: bool,
}

fn get_websocket_url(host: String, addr: SocketAddr, endpoint: &str) -> String {
    format!("ws://{}:{}/{}", host, addr.port(), endpoint)
}

pub fn build<'a>(config: Config, options: BuildOptions) -> Result<(), Error> {

    if options.live && options.release {
        return Err(
            Error::new(
                "Live reload is not available for release builds".to_string()))
    }

    let src = options.source.clone();
    let host = options.host.clone();
    let port = options.port.clone();
    let live = options.live.clone();

    let base_target = options.target.clone();

    //let mut target = options.source.clone();
    //if let Some(dir) = &options.directory {
        //target = dir.clone().to_path_buf();
    //}

    let mut generators = GeneratorMap::new();
    generators.load(src)?;

    loader::load(&options)?;

    let from = options.from.clone();

    let mut ctx = context::Context::new(config, options, generators);

    if !live {
        let mut builder = Builder::new(&ctx);
        builder.load_manifest()?;
        builder.build(&from, false)?;
        return builder.save_manifest()
    } else {
        let endpoint = utils::generate_id(16);

        let opts = ServeOptions {
            target: base_target.to_path_buf(),
            watch: Some(from.clone()),
            host: host.to_owned(),
            port: port.to_owned(),
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

            //options.livereload = Some(get_websocket_url(host, addr, &endpoint));

            ctx.livereload = Some(get_websocket_url(host, addr, &endpoint));

            // Do a full build before listening for filesystem changes
            let mut serve_builder = Builder::new(&ctx);
            if let Err(e) = serve_builder.register_templates_directory() {
                error!("{}", e);
                std::process::exit(1);
            }

            // WARN: must not load_manifest() here otherwise we can have
            // WARN: stale livereload endpoint URLs!

            if let Err(_) = serve_builder.load_manifest() {}

            let result = serve_builder.build(&from, true);

            match result {
                Ok(_) => {

                    // NOTE: only open the browser if initial build succeeds
                    open::that(&url).map(|_| ()).unwrap_or(());

                    #[cfg(feature = "watch")]
                    trigger_on_change(&from.clone(), move |paths, source_dir| {
                        info!("changed({}) in {}", paths.len(), source_dir.display());
                        debug!("files changed: {:?}", paths);
                        if let Ok(invalidation) = serve_builder.get_invalidation(paths) {
                            debug!("invalidation {:?}", invalidation);
                            if let Err(e) = serve_builder.invalidate(&from, invalidation) {
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

