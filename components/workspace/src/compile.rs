use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::info;

use compiler::context::Context;
use compiler::Compiler;
use config::{ProfileSettings, Config, RuntimeOptions};
use datasource::DataSourceMap;
use locale::Locales;

use crate::Result;

pub fn compile_project<P: AsRef<Path>>(
    project: P,
    args: &mut ProfileSettings,
    skip_last: bool) -> Result<Context> {

    let mut spaces: Vec<Config> = Vec::new();
    super::finder::find(project, true, &mut spaces)?;

    let length = spaces.len();

    let mut ctx: Context = Default::default();
    for (i, config) in spaces.into_iter().enumerate() {
        let mut dry_run = false;

        if skip_last && i == (length - 1) {
            dry_run = true;
        }

        ctx = compile(&config, args, dry_run)?;

        let write_redirects = args.write_redirects.is_some() && args.write_redirects.unwrap();
        if write_redirects {
            compiler::redirect::write()?;
        }
    }

    Ok(ctx)
}

pub fn compile(config: &Config, args: &mut ProfileSettings, dry_run: bool) -> Result<Context> {
    let opts = super::project::prepare(config, args)?;
    compile_one(config, opts, dry_run)
}

fn compile_one(config: &Config, opts: RuntimeOptions, dry_run: bool) -> Result<Context> {
    let mut ctx: Context = Default::default();
    let base_target = opts.target.clone();

    let mut locales = Locales::new(&config);
    locales.load(&config, &opts)?;

    //println!("Is multi {:?}", locales.is_multi());
    //println!("Is dry run {:?}", dry_run);

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
            copy.load(&config, &lang_opts)?;

            // FIXME: move the lang to RuntimeOptions
            copy.lang = lang.clone();

            //println!("Build for lang {:?}", copy.lang);

            ctx = load(copy, config.clone(), lang_opts)?;

            // NOTE: this old conditional will break multi-lingual builds
            // NOTE: when live reload is enabled. We need to find a better
            // NOTE: way to handle workspace builds with live reload and multi-lingual sites

            //if !dry_run {
                build(&ctx)?;
            //}
        }
    } else {
        ctx = load(locales, config.clone(), opts)?;
        if !dry_run {
            build(&ctx)?;
        }
    }
    Ok(ctx)
}

fn load(locales: Locales, config: Config, options: RuntimeOptions) -> Result<Context> {

    // Load data sources and create indices
    let datasource = DataSourceMap::load(&config, &options)?;

    // Load page template data
    loader::load(&options)?;

    // Set up the real context
    let ctx = Context::new(locales, config.clone());

    let runtime = runtime::runtime();
    let mut data = runtime.write().unwrap();
    data.config = config;
    data.options = options;
    data.datasource = datasource;

    Ok(ctx)
}

pub fn build(ctx: &Context) -> std::result::Result<Compiler, compiler::Error> {

    let runtime = runtime::runtime().read().unwrap();

    let mut builder = Compiler::new(ctx, runtime.options.clone());
    builder.manifest.load()?;

    let mut targets: Vec<PathBuf> = Vec::new();

    if let Some(ref paths) = runtime.options.settings.paths {
        builder.verify(paths)?;
        for p in paths {
            targets.push(p.clone());
        }
    } else {
        targets.push(runtime.options.source.clone());
    }

    builder.all(targets)?;
    builder.manifest.save()?;

    Ok(builder)
}
