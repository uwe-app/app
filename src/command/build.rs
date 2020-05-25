use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::build::Builder;
use crate::build::loader;
use crate::command::serve::*;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use crate::{Error};

use std::net::SocketAddrV4;
use std::net::Ipv4Addr;

use log::{info, debug, error};

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
}

impl BuildOptions {
    fn clone(&self) -> Self {
        let directory = if self.directory.is_some() {
            let val = self.directory.as_ref().unwrap().clone();
            Some(val)
        } else {
            None
        };
        let max_depth = if self.max_depth.is_some() {
            Some(self.max_depth.unwrap().clone())
        } else {
            None
        };
        let livereload = if self.livereload.is_some() {
            let val = self.livereload.as_ref().unwrap().clone();
            Some(val)
        } else {
            None
        };

        let tag = self.tag.clone();
        BuildOptions {
            source: self.source.clone(),
            output: self.output.clone(),
            target: self.target.clone(),
            release: self.release.clone(),
            directory,
            max_depth,
            follow_links: self.follow_links.clone(),
            strict: self.strict.clone(),
            clean_url: self.clean_url.clone(),
            tag,
            live: self.live.clone(),
            livereload,
            host: self.host.clone(),
            port: self.port.clone(),
        } 
    }
}

fn get_websocket_url(options: &BuildOptions, addr: SocketAddr, endpoint: &str) -> String {
    format!("ws://{}:{}/{}", options.host, addr.port(), endpoint)
}

fn do_build(target: PathBuf, options: BuildOptions) -> Result<(), Error> {
    let mut builder = Builder::new(&options);
    builder.build(&target)
}

pub fn build(mut options: BuildOptions) -> Result<(), Error> {
    if let Err(e) = loader::load(&options) {
        return Err(e)
    }

    // This is quite convoluted due to the nature of starting a
    // web server that blocks.
    //
    // The easy path is when `live` is false, we just build and 
    // finish the program.
    //
    // When `live` is enabled the flow is like this:
    //
    // 1) Start a listening thread `bind_handle` for the `tx` channel
    // 2) Call serve() to start the web server, pass the `tx` channel
    // 3) If the server binds then `bind_handle` stores the `SocketAddr` in `ADDR`
    // 4) It then proceeds to run a full build.
    // 5) When file changes are detected the closure passed to server() triggers
    // 6) It uses the `ADDR` that we are bound to in order to construct the correct 
    //    websocket URL.
    //
    // Much of this would be unnecessary if we only supported specific port arguments, 
    // but we also want to support ephemeral ports when `--port=0` so this allows us
    // to get back the actual port used and assign it to `livereload` so that templates will
    // connect to the correct websocket.
    //
    // Due to the use of multiple `move` closures we must copy the build options.

    let mut target = options.source.clone();
    if let Some(dir) = &options.directory {
        target = dir.clone();
    }

    if options.live {

        let watch_target = target.clone();

        // Must copy options due to use in multiple closures
        let mut copy = options.clone();

        let endpoint = utils::generate_id(16);
        let reload_endpoint = endpoint.clone();

        let opts = ServeOptions::new(
            options.target.clone(),
            target.clone(),
            options.host.to_owned(),
            options.port.to_owned(),
            endpoint.clone());

        // Create a channel to receive the bind address.
        let (tx, rx) = channel::<SocketAddr>();

        let _bind_handle = std::thread::spawn(move || {
            let addr = rx.recv().unwrap();

            let mut data = ADDR.lock().unwrap();
            *data = addr;

            let url = get_websocket_url(&options, addr, &endpoint);
            options.livereload = Some(url);

            do_build(target, options)
        });

        serve(opts, tx, move |paths, source_dir| {
            info!("changed({}) in {}", paths.len(), source_dir.display());
            debug!("files changed: {:?}", paths);

            let data = ADDR.lock().unwrap();
            let url = get_websocket_url(&copy, *data, &reload_endpoint);
            copy.livereload = Some(url);

            let mut builder = Builder::new(&copy);
            if let Err(e) = builder.register_templates_directory() {
                error!("{}", e);
                std::process::exit(1);
            }
            if let Ok(invalidation) = builder.get_invalidation(paths) {
                debug!("invalidation {:?}", invalidation);
                builder.invalidate(&watch_target, invalidation)?;
            }

            Ok(())
        })?;

    } else {
        do_build(target, options)?
    }

    Ok(())
}

