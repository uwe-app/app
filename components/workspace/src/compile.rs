use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::convert::TryInto;

use log::info;

use compiler::{Compiler, BuildContext};
use compiler::parser::Parser;
use config::{ProfileSettings, Config, RuntimeOptions};
use datasource::DataSourceMap;
use locale::Locales;

use collator::{CollateRequest, CollateResult, CollateInfo};
use collator::manifest::Manifest;
use collator::loader;

use crate::{Error, Result};
use crate::finder;

pub async fn compile_project<'a, P: AsRef<Path>>(
    project: P,
    args:&mut ProfileSettings) -> Result<(BuildContext, Locales)> {

    let mut spaces: Vec<Config> = Vec::new();
    finder::find(project, true, &mut spaces)?;

    let mut ctx = Default::default();
    for config in spaces.into_iter() {
        ctx = compile(&config, args).await?;
    }

    Ok(ctx)
}

pub async fn compile(config: &Config, args: &mut ProfileSettings) -> Result<(BuildContext, Locales)> {
    let opts = super::project::prepare(config, args)?;

    let write_redirects = opts.settings.write_redirects.is_some()
        && opts.settings.write_redirects.unwrap();

    let mut res = compile_one(config, opts).await;

    if let Ok((ref mut ctx, _)) = res {
        if write_redirects {
            compiler::redirect::write(ctx)?;
        }

        // Write the manifest for incremental builds
        if let Some(ref mut manifest) = ctx.collation.manifest {
            let manifest_file = get_manifest_file(&ctx.options);
            for (p, _) in ctx.collation.targets.iter() {
                manifest.touch(&p.to_path_buf());
            }
            Manifest::save(&manifest_file, manifest)?;
        }

    }
    
    res
}

async fn compile_one(config: &Config, opts: RuntimeOptions) -> Result<(BuildContext, Locales)> {

    let base_target = opts.target.clone();
    //let mut options = opts.clone();

    let mut locales: Locales = Default::default();
    locales.load(&config, &opts)?;

    let mut ctx = load(config.clone(), opts, None).await?;

    let mut previous_base = base_target.clone();

    if locales.is_multi() {
        for lang in locales.map.keys() {
            let locale_target = base_target.join(&lang);

            info!("lang {} -> {}", &lang, locale_target.display());

            if !locale_target.exists() {
                fs::create_dir_all(&locale_target)?;
            }

            // Keep the options target in sync for manifests
            ctx.options.target = locale_target.clone();

            // Rewrite the output paths and page languages
            ctx.collation.rewrite(&lang, &previous_base, &locale_target)?;

            previous_base = locale_target;

            build(&mut ctx, &locales).await?;
        }
    } else {
        build(&mut ctx, &locales).await?;
    };

    Ok((ctx, locales))
}

async fn load(
    //locales: Locales,
    config: Config,
    mut options: RuntimeOptions,
    lang: Option<String>) -> Result<BuildContext> {

    // Finalize the language for this pass
    options.lang = if let Some(lang) = lang {
        lang
    } else {
        config.lang.clone()
    };

    let should_collate = options.settings.should_collate();

    if should_collate {
        // Verify that files referenced by key in the pages
        // map exist on disc
        loader::verify(&config, &options)?;
    }

    // Set up the manifest for incremental builds
    let manifest_file = get_manifest_file(&options);
    let manifest: Option<Manifest> = if options.settings.is_incremental() {
        Some(Manifest::load(&manifest_file)?)
    } else {
        None
    };

    // Collate page data for later usage
    let req = CollateRequest {
        filter: false,
        config: &config,
        options: &options, 
    };

    let mut res = CollateResult::new(manifest);
    collator::walk(req, &mut res).await?;

    let mut collation: CollateInfo = res.try_into()?;

    if !collation.errors.is_empty() {
        // TODO: print all errors?
        let e = collation.errors.swap_remove(0);
        return Err(Error::Collator(e));
    }

    // Load data sources and create indices
    let datasource = DataSourceMap::load(&config, &options, &mut collation).await?;

    let mut cache = DataSourceMap::get_cache();

    DataSourceMap::assign(&config, &options, &mut collation, &datasource, &mut cache)?;
    DataSourceMap::expand(&config, &options, &mut collation, &datasource, &mut cache)?;

    // Set up the real context
    Ok(BuildContext::new(config, options, datasource, collation))
}

fn get_manifest_file(options: &RuntimeOptions) -> PathBuf {
    let mut manifest_file = options.base.clone();
    manifest_file.set_extension(config::JSON);
    manifest_file
}

pub async fn build<'a>(ctx: &'a mut BuildContext, locales: &'a Locales) -> std::result::Result<(Compiler<'a>, Parser<'a>), compiler::Error> {

    let parser = Parser::new(ctx, locales)?;
    let builder = Compiler::new(ctx);

    let mut targets: Vec<PathBuf> = Vec::new();

    if let Some(ref paths) = ctx.options.settings.paths {
        builder.verify(paths)?;
        for p in paths {
            targets.push(p.clone());
        }
    } else {
        targets.push(ctx.options.source.clone());
    }

    builder.all(&parser, targets).await?;

    Ok((builder, parser))
}
