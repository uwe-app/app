use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc::channel;

use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use config::BuildArguments;
use content;
use utils;

use crate::build::context::Context;
use crate::build::invalidator::Invalidator;
use crate::build::compiler::Compiler;
use crate::build::redirect;
use crate::{Error};


use crate::command::run::{self, ServeOptions};

use crate::ErrorCallback;
use crate::workspace;

fn get_websocket_url(host: String, addr: SocketAddr, endpoint: &str) -> String {
    format!("ws://{}:{}/{}", host, addr.port(), endpoint)
}

pub fn build_project<P: AsRef<Path>>(
    project: P,
    args: &BuildArguments,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let ctx = workspace::compile_project(project, args)?;
    if ctx.options.live {
        livereload(ctx, error_cb)?;
    }
    Ok(())
}

fn livereload(mut ctx: Context, error_cb: ErrorCallback) -> Result<(), Error> {
    let host = ctx.options.host.clone();
    let port = ctx.options.port.clone();

    let source = ctx.options.source.clone();
    let endpoint = utils::generate_id(16);

    let mut redirect_uris = None;

    if let Some(ref redirects) = ctx.config.redirect {
        redirect_uris = Some(redirect::collect(redirects)?);
    }

    let opts = ServeOptions {
        target: ctx.options.base.clone().to_path_buf(),
        watch: Some(source.clone()),
        host: host.to_owned(),
        port: port.to_owned(),
        endpoint: endpoint.clone(),
        open_browser: false,
        redirects: redirect_uris,
    };

    // Create a channel to receive the bind address.
    let (tx, rx) = channel::<(SocketAddr, Sender<Message>, String)>();

    // Spawn a thread to receive a notification on the `rx` channel
    // once the server has bound to a port
    std::thread::spawn(move || {
        // Get the socket address and websocket transmission channel
        let (addr, tx, url) = rx.recv().unwrap();

        let ws_url = get_websocket_url(host, addr, &endpoint);

        if let Err(e) = content::livereload::write(&ctx.options.target, &ws_url) {
            error_cb(Error::from(e));
            return;
        }

        ctx.livereload = Some(ws_url);

        let mut serve_builder = Compiler::new(&ctx);
        if let Err(e) = serve_builder.register_templates_directory() {
            error_cb(e);
        }

        // Prepare for incremental builds
        if let Err(_) = serve_builder.manifest.load() {}

        // NOTE: only open the browser if initial build succeeds
        open::that(&url).map(|_| ()).unwrap_or(());

        // Invalidator wraps the builder receiving filesystem change
        // notifications and sending messages over the `tx` channel
        // to connected websockets when necessary
        let mut invalidator = Invalidator::new(&ctx, serve_builder);
        if let Err(e) = invalidator.start(source, tx, &error_cb) {
            error_cb(e);
        }
    });

    // Start the webserver
    run::serve(opts, tx)?;

    Ok(())
}
