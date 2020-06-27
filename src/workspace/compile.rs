use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::info;

use crate::build::context::Context;
use crate::build::generator::GeneratorMap;
use crate::build::loader;
use crate::build::compiler::Compiler;
use crate::build::CompilerOptions;
use crate::config::{BuildArguments, Config};

use crate::Result;
use crate::locale::Locales;

use super::Workspace;

pub fn compile_project<P: AsRef<Path>>(project: P, args: &BuildArguments) -> Result<Context> {
    let mut spaces: Vec<Workspace> = Vec::new();
    super::finder::find(project, true, &mut spaces)?;

    let mut ctx: Context = Default::default();
    for mut space in spaces {
        ctx = compile_from(&mut space.config, &args)?;
    }

    // TODO: restore this behind a flag write-redirects
    //crate::build::redirect::write(&ctx)?;

    Ok(ctx)
}

pub fn compile_from(mut config: &mut Config, args: &BuildArguments) -> Result<Context> {
    let opts = super::project::prepare(&mut config, &args)?;
    compile(&config, opts)
}

pub fn compile(config: &Config, opts: CompilerOptions) -> Result<Context> {
    let mut ctx: Context = Default::default();
    //let opts = super::project::prepare(&mut config, &args)?;
    let base_target = opts.target.clone();
    let build_config = config.build.as_ref().unwrap();

    let mut locales = Locales::new(&config);
    locales.load(&config, &build_config.source)?;

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
            let mut copy = Locales::new(&config);
            copy.load(&config, &build_config.source)?;
            copy.lang = lang.clone();

            ctx = load(copy, config.clone(), lang_opts)?;
            build(&ctx)?;
        }
    } else {
        ctx = load(locales, config.clone(), opts)?;
        build(&ctx)?;
    }

    Ok(ctx)
}

fn load(locales: Locales, config: Config, options: CompilerOptions) -> Result<Context> {
    // Load generators
    let mut generators = GeneratorMap::new();
    generators.load(options.source.clone(), &config)?;

    // Load page template data
    loader::load(&config, &options.source)?;

    // Set up the context
    Ok(Context::new(locales, config, options, generators))
}

fn build(ctx: &Context) -> Result<()> {
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

    Ok(())
}

