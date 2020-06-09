use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use std::net::SocketAddrV4;
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use crate::config::Config;
use crate::build::Builder;
use crate::build::generator::GeneratorMap;
use crate::build::loader;
use crate::build::context;
use crate::build::invalidator::Invalidator;
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

type ErrorCallback = fn(Error);

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

pub fn build<'a>(config: Config, options: BuildOptions, error_cb: ErrorCallback) -> Result<(), Error> {

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

    let mut generators = GeneratorMap::new();
    generators.load(src, &config)?;

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
        let (tx, rx) = channel::<(SocketAddr, Sender<Message>, String)>();

        // Spawn a thread to receive a notification on the `rx` channel
        // once the server has bound to a port
        std::thread::spawn(move || {
            // Get the socket address and websocket transmission channel
            let (addr, tx, url) = rx.recv().unwrap();

            ctx.livereload = Some(get_websocket_url(host, addr, &endpoint));

            let mut serve_builder = Builder::new(&ctx);
            if let Err(e) = serve_builder.register_templates_directory() {
                error_cb(e);
            }

            // Prepare for incremental builds
            if let Err(_) = serve_builder.load_manifest() {}

            // Do a full build before listening for filesystem changes
            let result = serve_builder.build(&from, true);

            match result {
                Ok(_) => {
                    // NOTE: only open the browser if initial build succeeds
                    open::that(&url).map(|_| ()).unwrap_or(());

                    // Invalidator wraps the builder receiving filesystem change
                    // notifications and sending messages over the `tx` channel
                    // to connected websockets when necessary
                    let mut invalidator = Invalidator::new(&ctx, serve_builder);
                    if let Err(e) = invalidator.start(from, tx) {
                        error_cb(e);
                    }
                },
                Err(e) => {
                    error_cb(e);
                }
            }
        });

        // Start the webserver
        serve(opts, tx)?;
    }

    Ok(())
}
