use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::channel;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use log::info;

use crate::build::context::Context;
use crate::build::generator::GeneratorMap;
use crate::build::invalidator::Invalidator;
use crate::build::loader;
use crate::build::compiler::Compiler;
use crate::build::report::FileBuilder;
use crate::build::CompilerOptions;
use crate::command::serve::*;
use crate::config::{BuildArguments, Config};
use crate::{utils, Error};

use crate::ErrorCallback;
use crate::workspace::{self, Workspace};

use crate::locale::Locales;

fn get_websocket_url(host: String, addr: SocketAddr, endpoint: &str) -> String {
    format!("ws://{}:{}/{}", host, addr.port(), endpoint)
}

pub fn build_project<P: AsRef<Path>>(
    project: P,
    args: &BuildArguments,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let mut spaces: Vec<Workspace> = Vec::new();
    workspace::finder::find(project, true, &mut spaces)?;
    build_workspaces(spaces, args, error_cb)
}

fn build_workspaces(
    spaces: Vec<Workspace>,
    args: &BuildArguments,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let mut ctx: Context = Default::default();

    for mut space in spaces {
        let opts = workspace::project::prepare(&mut space.config, &args)?;
        let base_target = opts.target.clone();
        let build_config = space.config.build.as_ref().unwrap();

        let mut locales = Locales::new(&space.config);
        locales.load(&space.config, &build_config.source)?;

        if locales.is_multi() {
            for lang in locales.map.keys() {
                let mut lang_opts = opts.clone();

                let mut locale_target = base_target.clone();
                locale_target.push(&lang);

                info!("lang {} -> {}", &lang, locale_target.display());

                if !locale_target.exists() {
                    fs::create_dir_all(&locale_target)?;
                }

                lang_opts.target = locale_target;

                // FIXME: prevent loading all the locales again!?
                let mut copy = Locales::new(&space.config);
                copy.load(&space.config, &build_config.source)?;
                copy.lang = lang.clone();

                ctx = load(copy, space.config.clone(), lang_opts)?;
                build(&ctx)?;
            }
        } else {
            ctx = load(locales, space.config, opts)?;
            build(&ctx)?;
        }
    }

    crate::build::redirect::write(&ctx)?;

    if ctx.options.live {
        livereload(ctx, error_cb)?;
    }

    Ok(())
}

fn load(locales: Locales, config: Config, options: CompilerOptions) -> Result<Context, Error> {
    // Load generators
    let mut generators = GeneratorMap::new();
    generators.load(options.source.clone(), &config)?;

    // Load page template data
    loader::load(&config, &options.source)?;

    // Set up the context
    Ok(Context::new(locales, config, options, generators))
}

fn build(ctx: &Context) -> Result<(), Error> {

    let mut builder = Compiler::new(ctx);
    builder.manifest.load()?;

    let mut targets: Vec<PathBuf> = Vec::new();

    if let Some(ref paths) = ctx.options.paths {
        builder.verify(paths)?;
        for p in paths {
            targets.push(p.clone());
        }
    } else {
        targets.push(ctx.options.source.clone());
    }

    builder.all(targets)?;

    builder.manifest.save()?;

    //let mut file_builder = FileBuilder::new(true, ctx.options.base.clone(), true, true, None);
    //file_builder.walk()?;

    Ok(())
}

fn livereload(mut ctx: Context, error_cb: ErrorCallback) -> Result<(), Error> {
    let host = ctx.options.host.clone();
    let port = ctx.options.port.clone();

    let source = ctx.options.source.clone();
    let endpoint = utils::generate_id(16);

    let opts = ServeOptions {
        target: ctx.options.base.clone().to_path_buf(),
        watch: Some(source.clone()),
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

        let ws_url = get_websocket_url(host, addr, &endpoint);

        if let Err(e) = crate::content::livereload::write(&ctx.options.target, &ws_url) {
            error_cb(e);
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
    serve(opts, tx)?;

    Ok(())
}
