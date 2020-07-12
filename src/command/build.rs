use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc::channel;

use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use compiler::context::Context;
use compiler::invalidator::Invalidator;
use compiler::redirect;
use compiler::ErrorCallback;
use config::ProfileSettings;

use crate::command::run::{self, ServeOptions};
use crate::Error;

fn get_websocket_url(host: String, addr: SocketAddr, endpoint: &str) -> String {
    format!("ws://{}:{}/{}", host, addr.port(), endpoint)
}

pub fn compile<P: AsRef<Path>>(
    project: P,
    args: &ProfileSettings,
    error_cb: ErrorCallback,
) -> Result<(), Error> {

    let live = args.live.is_some() && args.live.unwrap();
    let ctx = workspace::compile_project(project, args, live)?;

    if live {
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

        if let Err(e) = livereload::write(&ctx.config, &ctx.options.target, &ws_url) {
            error_cb(compiler::Error::from(e));
            return;
        }

        ctx.livereload = Some(ws_url);

        let built = workspace::build(&ctx);
        match built {
            Ok(mut compiler) => {
                //let mut serve_builder = workspace::build(&ctx);
                if let Err(e) = compiler.register_templates_directory() {
                    error_cb(e);
                }

                // Prepare for incremental builds
                if let Err(_) = compiler.manifest.load() {}

                // NOTE: only open the browser if initial build succeeds
                open::that(&url).map(|_| ()).unwrap_or(());

                // Invalidator wraps the builder receiving filesystem change
                // notifications and sending messages over the `tx` channel
                // to connected websockets when necessary
                let mut invalidator = Invalidator::new(&ctx, compiler);
                if let Err(e) = invalidator.start(source, tx, &error_cb) {
                    error_cb(e);
                }
            
            },
            Err(e) => {
                error_cb(e);
            }
        }

    });

    // Start the webserver
    run::serve(opts, tx)?;

    Ok(())
}
