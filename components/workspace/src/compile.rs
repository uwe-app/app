use std::convert::TryInto;
use std::fs::{self, File};
use std::path::Path;
use std::path::PathBuf;

use human_bytes::human_bytes;
use log::info;

use cache::CacheComponent;
use compiler::redirect;
use compiler::parser::Parser;
use compiler::{BuildContext, Compiler, ParseData};
use config::{Config, ProfileSettings, RuntimeOptions};
use config::sitemap::{SiteMapIndex, SiteMapFile, SiteMapEntry};
use datasource::synthetic;
use datasource::DataSourceMap;
use locale::Locales;
use search::{compile as compile_index, intermediate, Index, IntermediateEntry};

use collator::loader;
use collator::manifest::Manifest;
use collator::{CollateInfo, CollateRequest, CollateResult};

use crate::finder;
use crate::{Error, Result};

pub async fn compile_project<'a, P: AsRef<Path>>(
    project: P,
    args: &mut ProfileSettings,
) -> Result<(BuildContext, Locales)> {
    let mut spaces: Vec<Config> = Vec::new();
    finder::find(project, true, &mut spaces)?;

    let mut ctx = Default::default();
    for mut config in spaces.iter_mut() {
        ctx = compile(&mut config, args).await?;
    }

    Ok(ctx)
}

pub async fn compile(
    config: &mut Config,
    args: &mut ProfileSettings,
) -> Result<(BuildContext, Locales)> {

    // Finalize the runtime options
    let mut opts = super::project::prepare(config, args)?;

    let write_redirects =
        opts.settings.write_redirects.is_some() && opts.settings.write_redirects.unwrap();

    let mut res = render(config, &mut opts).await;

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

async fn render(config: &mut Config, opts: &mut RuntimeOptions) -> Result<(BuildContext, Locales)> {
    let base_target = opts.target.clone();

    let mut locales: Locales = Default::default();
    locales.load(&config, &opts)?;
    let locale_map = locales.get_locale_map(&config.lang)?;

    opts.locales = locale_map.clone();

    fetch_cache_lazy(config, &opts)?;

    let (collation, datasource) = collate(config, opts).await?;
    let mut ctx = BuildContext::new(
        config.clone(), opts.clone(), datasource, collation);

    let mut previous_base = base_target.clone();

    for lang in locale_map.map.keys() {
        // When we have multiple languages we need to rewrite paths
        // on each iteration for each specific language
        if lang_res.multi {
            let locale_target = base_target.join(&lang);
            info!("lang {} -> {}", &lang, locale_target.display());

            if !locale_target.exists() {
                fs::create_dir_all(&locale_target)?;
            }

            // Keep the target language in sync
            ctx.options.lang = lang.clone();

            // Keep the options target in sync for manifests
            ctx.options.target = locale_target.clone();

            // Rewrite the output paths and page languages
            ctx.collation
                .rewrite(&lang, &previous_base, &locale_target)?;

            previous_base = locale_target;
        }

        prepare(&mut ctx)?;
        let (_, _, parse_list) = build(&mut ctx, &locales).await?;
        finish(&mut ctx, parse_list)?;
    }

    write_robots_file(&mut ctx)?;

    Ok((ctx, locales))
}

async fn collate(
    //locales: Locales,
    config: &mut Config,
    options: &RuntimeOptions,
) -> Result<(CollateInfo, DataSourceMap)> {

    // FIXME: remove this test and flag, to do with mixing
    // FIXME: functionality and sources in build profiles
    // FIXME: which should not be allowed, see the blog/readme profile.
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

    // Set up the cache for data source queries
    let mut cache = DataSourceMap::get_cache();

    // Map permalink redirects
    if !collation.permalinks.is_empty() {
        // Must have some redirects
        if let None = config.redirect {
            config.redirect = Some(Default::default());
        }

        if let Some(redirects) = config.redirect.as_mut() {
            for (permalink, href) in collation.permalinks.iter() {
                let key = permalink.to_string() ;
                if redirects.contains_key(&key) {
                    return Err(Error::RedirectPermalinkCollision(key));
                }
                redirects.insert(key, href.to_string());
            }
        }
    }

    // Validate the redirects
    if let Some(ref redirects) = config.redirect {
        redirect::validate(redirects)?;
    }

    // Copy the search runtime files if we need them
    synthetic::search(&config, &options, &mut collation)?;
    // Create feed pages
    synthetic::feed(&config, &options, &mut collation)?;

    // Perform pagination
    synthetic::pages(&config, &options, &mut collation, &datasource, &mut cache)?;
    // Create pages for iterators
    synthetic::each(&config, &options, &mut collation, &datasource, &mut cache)?;
    // Assign data from queries
    synthetic::assign(&config, &options, &mut collation, &datasource, &mut cache)?;

    // Collate the series data
    collator::series(&config, &options, &mut collation)?;

    Ok((collation, datasource))
}

fn get_manifest_file(options: &RuntimeOptions) -> PathBuf {
    let mut manifest_file = options.base.clone();
    manifest_file.set_extension(config::JSON);
    manifest_file
}


fn fetch_cache_lazy(config: &Config, opts: &RuntimeOptions) -> Result<()> {
    let mut components: Vec<CacheComponent> = Vec::new();

    if config.syntax.is_some() {
        if config.is_syntax_enabled(&opts.settings.name) {
            let syntax_dir = cache::get_syntax_dir()?;
            if !syntax_dir.exists() {
                components.push(CacheComponent::Syntax);
            }
        }
    }

    if let Some(ref search) = config.search {
        let fetch_search_runtime = search.bundle.is_some() && search.bundle.unwrap();
        if fetch_search_runtime {
            let search_dir = cache::get_search_dir()?;
            if !search_dir.exists() {
                components.push(CacheComponent::Search);
            }
        }
    }

    if config.feed.is_some() {
        let feed_dir = cache::get_feed_dir()?;
        if !feed_dir.exists() {
            components.push(CacheComponent::Feed);
        }
    }

    if !components.is_empty() {
        let prefs = preference::load()?;
        cache::update(&prefs, components)?;
    }

    Ok(())
}

fn prepare<'a>(ctx: &'a mut BuildContext) -> Result<()> {

    if let Some(ref syntax_config) = ctx.config.syntax {
        if ctx.config.is_syntax_enabled(&ctx.options.settings.name) {
            let syntax_dir = cache::get_syntax_dir()?;
            info!("Syntax highlighting on");
            syntax::setup(&syntax_dir, syntax_config)?;
        }
    }

    Ok(())
}

fn create_search_indices<'a>(ctx: &'a mut BuildContext, parse_list: &Vec<ParseData>) -> Result<()> {
    let include_index = ctx.options.settings.should_include_index();
    if let Some(ref search) = ctx.config.search {
        for (_id, search) in search.items.iter() {
            let mut intermediates: Vec<IntermediateEntry> = Vec::new();
            info!("Prepare search index ({})", parse_list.len());
            for parse_data in parse_list {
                if let Some(ref extraction) = parse_data.extract {
                    let href = ctx.collation.links.sources.get(&parse_data.file);

                    let buffer = extraction.to_chunk_string();
                    let title = if let Some(ref title) = extraction.title { title } else { "" };
                    let mut url = if let Some(ref href) = href { href } else { "" };

                    if !include_index && url.ends_with(config::INDEX_HTML) {
                        url = url.trim_end_matches(config::INDEX_HTML);
                    }

                    if !search.filter(url) {
                        continue;
                    }

                    intermediates.push(intermediate(&buffer, title, url, Default::default()));
                }
            }

            info!("Compile search index ({})", intermediates.len());
            let idx: Index = compile_index(intermediates);
            let index_file = search.get_output_path(&ctx.options.target);
            info!("Write search index to {}", index_file.display());
            let bytes_written = search::writer::write(&idx, index_file)?;
            info!("Search index {}", human_bytes(bytes_written as f64));
        }
    }
    Ok(())
}

fn create_site_map<'a>(ctx: &'a mut BuildContext, parse_list: &Vec<ParseData>) -> Result<()> {

    if let Some(ref sitemap) = ctx.options.settings.sitemap {
        if ctx.options.settings.robots.is_none() {
            ctx.options.settings.robots = Some(Default::default());
        }

        // How many entries per chunk window?
        let entries = sitemap.entries.as_ref().unwrap();

        // Base canonical URL
        let base = ctx.options.get_canonical_url(&ctx.config, true)?;

        // Create the top-level index of all sitemaps
        let folder = sitemap.name.as_ref().unwrap().to_string();
        let mut idx = SiteMapIndex::new(base.clone(), folder.clone());

        let base_folder = ctx.options.target.join(&folder);

        if !base_folder.exists() {
            fs::create_dir_all(&base_folder)?;
        }

        for (count, window) in parse_list.chunks(*entries).enumerate() {
            let href = format!("{}.xml", count + 1);
            let mut sitemap = SiteMapFile {href, entries: vec![]};
            let sitemap_path = base_folder.join(&sitemap.href);
            sitemap.entries = window
                .iter()
                // NOTE: quick hack to ignore error file, needs stronger logic
                .filter(|d| !d.file.ends_with("404.html"))
                .map(|d| {
                    // Get the href to use to build the location
                    let href = ctx.collation.links.sources.get(&d.file).unwrap();
                    // Get the last modification data from the page
                    let page = ctx.collation.pages.get(&d.file).unwrap();
                    // Generate the absolute location
                    let location = base.join(href).unwrap();
                    let lastmod = page.lastmod();
                    SiteMapEntry {location, lastmod}
                }).collect();

            let map_file = File::create(&sitemap_path)?;
            sitemap.to_writer(map_file)?;

            // Add the file to the index
            idx.maps.push(sitemap);
        }

        // Write out the master index file
        let idx_path = base_folder.join(config::sitemap::FILE);
        let idx_file = File::create(&idx_path)?;
        idx.to_writer(idx_file)?;

        let sitemap_url = idx.to_location();
        info!("Sitemap {} ({})", sitemap_url.to_string(), idx.maps.len());
        //info!("Sitemap {}", idx_path.display());

        // Update robots config to include the sitemap
        if let Some(ref mut robots) = ctx.options.settings.robots.as_mut() {
            robots.sitemaps.push(sitemap_url);
        }
    }

    Ok(())
}

fn write_robots_file<'a>(ctx: &'a mut BuildContext) -> Result<()> {
    if let Some(ref robots) = ctx.options.settings.robots {
        // NOTE: robots must always be at the root regardless
        // NOTE: of multi-lingual support so we use `base` rather
        // NOTE: than the `target`
        let robots_file = ctx.options.base.join(config::robots::FILE);
        utils::fs::write_string(&robots_file, robots.to_string())?;
        info!("Robots {}", robots_file.display());
    }
    Ok(())
}

fn finish<'a>(ctx: &'a mut BuildContext, parse_list: Vec<ParseData>) -> Result<()> {
    create_search_indices(ctx, &parse_list)?;
    create_site_map(ctx, &parse_list)?;
    Ok(())
}

async fn build<'a>(
    ctx: &'a mut BuildContext,
    locales: &'a Locales,
) -> std::result::Result<(Compiler<'a>, Parser<'a>, Vec<ParseData>), compiler::Error> {
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
