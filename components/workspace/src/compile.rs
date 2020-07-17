use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::convert::TryInto;

use log::info;

use compiler::{Compiler, BuildContext};
use config::{ProfileSettings, Config, RuntimeOptions};
use datasource::DataSourceMap;
use locale::Locales;

use collator::{CollateRequest, CollateResult, CollateInfo};

use crate::{Error, Result};

pub async fn compile_project<P: AsRef<Path>>(
    project: P,
    args: &mut ProfileSettings,
    skip_last: bool) -> Result<BuildContext> {

    let mut spaces: Vec<Config> = Vec::new();
    super::finder::find(project, true, &mut spaces)?;

    let length = spaces.len();

    let mut ctx: BuildContext = Default::default();
    for (i, config) in spaces.into_iter().enumerate() {
        let mut dry_run = false;

        if skip_last && i == (length - 1) {
            dry_run = true;
        }

        ctx = compile(&config, args, dry_run).await?;

        let write_redirects = args.write_redirects.is_some() && args.write_redirects.unwrap();
        if write_redirects {
            compiler::redirect::write(&ctx)?;
        }
    }

    Ok(ctx)
}

pub async fn compile(config: &Config, args: &mut ProfileSettings, dry_run: bool) -> Result<BuildContext> {
    let opts = super::project::prepare(config, args)?;
    compile_one(config, opts, dry_run).await
}

async fn compile_one(config: &Config, opts: RuntimeOptions, dry_run: bool) -> Result<BuildContext> {
    let mut ctx: BuildContext = Default::default();
    let base_target = opts.target.clone();

    let mut locales: Locales = Default::default();
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
            let mut copy: Locales = Default::default();
            copy.load(&config, &lang_opts)?;

            ctx = load(copy, config.clone(), lang_opts, Some(lang.clone())).await?;

            // NOTE: this old conditional will break multi-lingual builds
            // NOTE: when live reload is enabled. We need to find a better
            // NOTE: way to handle workspace builds with live reload and multi-lingual sites

            //if !dry_run {
                build(&mut ctx)?;
            //}
        }
    } else {
        ctx = load(locales, config.clone(), opts, None).await?;
        if !dry_run {
            build(&mut ctx)?;
        }
    }
    Ok(ctx)
}

async fn load(
    locales: Locales,
    config: Config,
    mut options: RuntimeOptions,
    lang: Option<String>) -> Result<BuildContext> {

    // Verify that files referenced by key in the pages
    // map exist on disc
    loader::verify(&config, &options)?;

    // Collate page data for later usage
    let req = CollateRequest {filter: false, config: &config, options: &options};
    let mut res = CollateResult::new();
    collator::walk(req, &mut res).await?;

    let mut collation: CollateInfo = res.try_into()?;

    if !collation.errors.is_empty() {
        let e = collation.errors.swap_remove(0);
        return Err(Error::Collator(e));
    }

    // Load data sources and create indices
    let datasource = DataSourceMap::load(&config, &options, &collation).await?;

    // Finalize the language for this pass
    options.lang = if let Some(lang) = lang {
        lang
    } else {
        config.lang.clone()
    };

    // Set up the real context
    Ok(BuildContext::new(config, options, datasource, locales, collation))
}

pub fn build(ctx: &BuildContext) -> std::result::Result<Compiler<'_>, compiler::Error> {

    let builder = Compiler::new(ctx)?;
    //builder.manifest.load()?;

    let mut targets: Vec<PathBuf> = Vec::new();

    if let Some(ref paths) = ctx.options.settings.paths {
        builder.verify(paths)?;
        for p in paths {
            targets.push(p.clone());
        }
    } else {
        targets.push(ctx.options.source.clone());
    }

    builder.all(targets)?;

    //builder.manifest.save()?;

    Ok(builder)
}
