use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::convert::TryInto;

use log::info;
use human_bytes::human_bytes;

use cache::CacheComponent;
use compiler::{Compiler, BuildContext, ParseData};
use compiler::parser::Parser;
use config::{ProfileSettings, Config, RuntimeOptions};
use datasource::DataSourceMap;
use datasource::synthetic;
use locale::Locales;
use search::{Index, IntermediateEntry, intermediate, compile as compile_index};

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

            prepare(&mut ctx)?;
            let (_, _, parse_list) = build(&mut ctx, &locales).await?;
            finish(&mut ctx, parse_list)?;
        }
    } else {
        prepare(&mut ctx)?;
        let (_, _, parse_list) = build(&mut ctx, &locales).await?;
        finish(&mut ctx, parse_list)?;
        //build(&mut ctx, &locales).await?;
        //finish(&mut ctx)?;
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

    // Copy the search runtime files if we need them
    synthetic::search(&config, &options, &mut collation)?;

    synthetic::pages(&config, &options, &mut collation, &datasource, &mut cache)?;
    synthetic::each(&config, &options, &mut collation, &datasource, &mut cache)?;
    synthetic::assign(&config, &options, &mut collation, &datasource, &mut cache)?;

    // Collate the series data
    collator::series(&config, &options, &mut collation)?;

    // Set up the real context
    Ok(BuildContext::new(config, options, datasource, collation))
}

fn get_manifest_file(options: &RuntimeOptions) -> PathBuf {
    let mut manifest_file = options.base.clone();
    manifest_file.set_extension(config::JSON);
    manifest_file
}

fn prepare<'a>(ctx: &'a mut BuildContext) -> Result<()> {
    if let Some(ref syntax_config) = ctx.config.syntax {
        if ctx.config.is_syntax_enabled(&ctx.options.settings.name) {
            let prefs = preference::load()?;
            let syntax_dir = cache::get_syntax_dir()?;
            if !syntax_dir.exists() {
                cache::update(&prefs, vec![CacheComponent::Syntax])?;
            }
            info!("Syntax highlighting on");
            syntax::setup(&syntax_dir, syntax_config)?;
        }
    }

    if let Some(ref search) = ctx.config.search {
        let fetch_search_runtime = search.copy_runtime.is_some() && search.copy_runtime.unwrap();
        if fetch_search_runtime {
            let prefs = preference::load()?;
            let search_dir = cache::get_search_dir()?;
            if !search_dir.exists() {
                cache::update(&prefs, vec![CacheComponent::Search])?;
            }
        }
    }

    Ok(())
}


fn finish<'a>(ctx: &'a mut BuildContext, parse_list: Vec<ParseData>) -> Result<()> {

    let include_index = ctx.options.settings.should_include_index();

    if let Some(ref search) = ctx.config.search {
        let output = search.output.as_ref().unwrap();

        let mut intermediates: Vec<IntermediateEntry> = Vec::new();
        info!("Prepare search index ({})", parse_list.len());
        // TODO: configure the pass through config
        for parse_data in parse_list {
            let extraction = parse_data.extract.as_ref().unwrap();

            // TODO: when not include_index strip the index.html from the href
            let href = ctx.collation.links.sources.get(&parse_data.file);

            let buffer = extraction.to_chunk_string();
            let title = if let Some(ref title) = extraction.title { title } else { "" };
            let mut url = if let Some(ref href) = href { href } else { "" };
            if !include_index && url.ends_with(config::INDEX_HTML) {
                url = url.trim_end_matches(config::INDEX_HTML);
            }

            //println!("Title {}", title);
            //println!("Buffer {}", &buffer);

            intermediates.push(
                intermediate(&buffer, title, url, Default::default()));
        }

        //println!("{:#?}", &intermediates);

        info!("Compile search index ({})", intermediates.len());
        let idx: Index = compile_index(intermediates);
        let index_file = ctx.options.target.join(output);
        info!("Write search index to {}", index_file.display());
        let bytes_written = idx.write(index_file, false)?;
        info!("Search index {}", human_bytes(bytes_written as f64));

    }

    Ok(())
}

async fn build<'a>(ctx: &'a mut BuildContext, locales: &'a Locales)
    -> std::result::Result<(Compiler<'a>, Parser<'a>, Vec<ParseData>), compiler::Error> {

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

    let parse_list = builder.all(&parser, targets).await?;
    Ok((builder, parser, parse_list))
}
