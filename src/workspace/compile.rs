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
use crate::locale::Locales;

use super::Workspace;

pub fn compile<P: AsRef<Path>>(
    project: P,
    args: &BuildArguments,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let mut spaces: Vec<Workspace> = Vec::new();
    super::finder::find(project, true, &mut spaces)?;
    compile_workspaces(spaces, args, error_cb)
}

fn compile_workspaces(
    spaces: Vec<Workspace>,
    args: &BuildArguments,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let mut ctx: Context = Default::default();

    for mut space in spaces {
        let opts = super::project::prepare(&mut space.config, &args)?;
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

    // FIXME: restore this

    //if ctx.options.live {
        //livereload(ctx, error_cb)?;
    //}

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

