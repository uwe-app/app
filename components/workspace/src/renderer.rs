use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use log::{debug, info};
use url::Url;

use human_bytes::human_bytes;

use collator::{builder::PageBuilder, resource::Resource};
use collections::{synthetic, CollectionsMap, QueryCache};
use compiler::{
    compile, parser::Parser, run, BuildContext, CompilerOutput, ParseData,
};
use config::{
    hook::HookConfig,
    plugin::dependency::DependencyTarget,
    profile::Profiles,
    sitemap::{SiteMapEntry, SiteMapFile, SiteMapIndex},
};
use locale::{LocaleName, Locales};
use search::{
    compile as compile_index, intermediate, Index, IntermediateEntry,
};

use crate::{hook, manifest::Manifest, Result};

#[derive(Clone)]
pub struct RenderOptions {
    pub target: RenderTarget,
    pub filter: RenderFilter,
    pub reload_data: bool,
    pub sitemap: bool,
    pub search_index: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            target: Default::default(),
            filter: Default::default(),
            reload_data: false,
            sitemap: true,
            search_index: true,
        }
    }
}

impl RenderOptions {
    pub fn new_file_lang(
        file: PathBuf,
        lang: String,
        reload_data: bool,
        sitemap: bool,
        search_index: bool,
    ) -> Self {
        Self {
            target: RenderTarget::File(file),
            filter: RenderFilter::One(lang),
            reload_data,
            sitemap,
            search_index,
        }
    }

    pub fn file(&self) -> Option<&PathBuf> {
        if let RenderTarget::File(ref path) = self.target {
            return Some(path);
        }
        None
    }
}

#[derive(Clone)]
pub enum RenderTarget {
    /// Render everything for this locale.
    All,
    /// Render a single file.
    File(PathBuf),
}

impl Default for RenderTarget {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Clone)]
pub enum RenderFilter {
    /// Render every locale.
    All,
    /// Render a single locale.
    One(LocaleName),
}

impl Default for RenderFilter {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Default)]
pub struct Sources {
    pub filters: Option<Vec<PathBuf>>,
}

#[derive(Debug, Default)]
pub struct CompilerInput {
    pub sources: Arc<Sources>,
    pub context: Arc<BuildContext>,
    pub locales: Arc<Locales>,
    pub collections: Arc<RwLock<CollectionsMap>>,
    pub manifest: Option<Arc<RwLock<Manifest>>>,
}

#[derive(Debug)]
pub struct RenderResult {
    pub sitemap: Option<Url>,
}

/// Renderer for a single language.
#[derive(Debug)]
pub struct Renderer {
    pub info: CompilerInput,
}

impl Renderer {
    pub fn new(info: CompilerInput) -> Self {
        Self { info }
    }

    /// Render a locale for a project.
    pub async fn render(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        render_options: &RenderOptions,
    ) -> Result<RenderResult> {
        let mut output: CompilerOutput = Default::default();

        match render_options.target {
            RenderTarget::All => {
                self.build(parser, render_options, &mut output).await?;
            }
            RenderTarget::File(ref path) => {
                let options = &self.info.context.options;
                let types = options.settings.types.as_ref().unwrap();
                // WARN: Must test the file path is a valid page before reloading data
                // WARN: otherwise binary files may be treated as pages and we would
                // WARN: inadvertently try to parse front matter from a binary file!
                if render_options.reload_data
                    && options.has_parse_file_match(path, types)
                {
                    self.reload(path)?;
                }
                self.one(parser, path).await?;
            }
        }

        Ok(RenderResult {
            sitemap: self.finish(output, render_options)?,
        })
    }

    /// Reload the data for a single page.
    pub fn reload<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let collation = self.info.context.collation.read().unwrap();

        let key = Arc::new(path.as_ref().to_path_buf());
        let path_buf = &*key;
        let mut info = collation.fallback.write().unwrap();
        let layout_name = collator::layout_name(&self.info.context.options);

        let plugins = self.info.context.plugins.as_deref();

        let builder = PageBuilder::new(
            &mut info,
            &self.info.context.config,
            &self.info.context.options,
            plugins,
            &key,
            path.as_ref(),
        )
        .compute()?
        .layout(layout_name)?
        .queries()?
        .seal()?
        .scripts()?
        .styles()?
        .layouts()?
        // WARN: calling link() will create a collision!
        //.link()?
        // WARN: calling permalinks() will create a collision!
        //.permalinks()?
        .feeds()?;

        let (_, _, _, computed_page) = builder.build();

        drop(info);

        if let Some(page_lock) = collation.resolve(path_buf) {
            let mut page_write = page_lock.write().unwrap();

            *page_write = computed_page;

            // Update collections query assignments
            let collate_info = collation.fallback.read().unwrap();
            let collections_map = self.info.collections.read().unwrap();
            let mut query_cache = QueryCache::new();
            synthetic::assign_page_lookup(
                &collate_info,
                &collections_map,
                &mut query_cache,
                &key,
                &mut page_write,
            )?;
        }

        Ok(())
    }

    fn finish(
        &self,
        output: CompilerOutput,
        render_options: &RenderOptions,
    ) -> Result<Option<Url>> {
        if render_options.search_index {
            self.create_search_indices(&output.data)?;
        }

        if render_options.sitemap {
            Ok(self.create_site_map(&output.data)?)
        } else {
            Ok(None)
        }
    }

    fn create_search_indices(&self, parse_list: &Vec<ParseData>) -> Result<()> {
        let ctx = &self.info.context;
        let collation = ctx.collation.read().unwrap();
        let include_index = ctx.options.settings.should_include_index();
        if let Some(ref search) = ctx.config.search {
            for (_id, search) in search.items.iter() {
                let mut intermediates: Vec<IntermediateEntry> = Vec::new();
                info!("Prepare search index ({})", parse_list.len());
                for parse_data in parse_list {
                    if let Some(ref extraction) = parse_data.extract {
                        let href = collation.get_link_href(&parse_data.file);

                        let buffer = extraction.to_chunk_string();
                        let title = if let Some(ref title) = extraction.title {
                            title
                        } else {
                            ""
                        };
                        let mut url =
                            if let Some(ref href) = href { href } else { "" };

                        if !include_index && url.ends_with(config::INDEX_HTML) {
                            url = url.trim_end_matches(config::INDEX_HTML);
                        }

                        if !search.matcher.filter(url) {
                            continue;
                        }

                        intermediates.push(intermediate(
                            &buffer,
                            title,
                            url,
                            Default::default(),
                        ));
                    }
                }

                info!("Compile search index ({})", intermediates.len());
                let idx: Index = compile_index(intermediates);
                let index_file =
                    search.get_output_path(collation.get_path().as_ref());

                // If there are path filters for compiling specific files
                // we are not guaranteed that the parent directory will exist!
                if let Some(parent) = index_file.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }

                info!("Write search index to {}", index_file.display());
                let bytes_written = search::writer::write(&idx, index_file)?;
                info!("Search index {}", human_bytes(bytes_written as f64));
            }
        }
        Ok(())
    }

    fn create_site_map(
        &self,
        parse_list: &Vec<ParseData>,
    ) -> Result<Option<Url>> {
        let ctx = &self.info.context;
        let collation = ctx.collation.read().unwrap();

        let mut res: Option<Url> = None;

        let sitemap = ctx.config.sitemap();
        if sitemap.profiles().is_match(ctx.options.profile()) {
            // How many entries per chunk window?
            let entries = sitemap.entries.as_ref().unwrap();

            let with_lang = if ctx.locales.is_multi_lingual() {
                Some(collation.get_lang().to_string())
            } else {
                None
            };

            // Base canonical URL
            let base = ctx
                .options
                .get_canonical_url(&ctx.config, with_lang.as_ref())?;

            // Create the top-level index of all sitemaps
            let folder = sitemap.name.as_ref().unwrap().to_string();
            let mut idx = SiteMapIndex::new(base.clone(), folder.clone());

            let base_folder = collation.get_path().join(&folder);

            if !base_folder.exists() {
                fs::create_dir_all(&base_folder)?;
            }

            let err_name = OsStr::new("404");

            for (count, window) in parse_list.chunks(*entries).enumerate() {
                let href = format!("{}.xml", count + 1);
                let mut sitemap = SiteMapFile {
                    href,
                    entries: vec![],
                };
                let sitemap_path = base_folder.join(&sitemap.href);
                sitemap.entries = window
                    .iter()
                    // NOTE: quick hack to ignore error file, needs stronger logic
                    .filter(|d| {
                        if Some(err_name) == d.file.file_stem() {
                            return false;
                        }
                        true
                    })
                    .map(|d| {
                        // Get the href to use to build the location
                        let href = collation.get_link_href(&d.file).unwrap();
                        // Get the last modification data from the page
                        let page = collation.resolve(&d.file).unwrap();
                        let page = &*page.read().unwrap();
                        // Generate the absolute location
                        let location = base.join(href.as_ref()).unwrap();
                        let lastmod = page.lastmod();
                        SiteMapEntry { location, lastmod }
                    })
                    .collect();

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

            res = Some(sitemap_url);
        }

        Ok(res)
    }

    pub(crate) async fn run_hook(
        &self,
        hook: &HookConfig,
        changed: Option<&PathBuf>,
    ) -> Result<()> {
        Ok(hook::run(&self.info.context, vec![hook], changed)?)
    }

    async fn run_before_hooks(&self) -> Result<()> {
        if let Some(ref hooks) = self.info.context.config.hooks() {
            hook::run(
                &self.info.context,
                hook::collect(
                    hooks.exec(),
                    hook::Phase::Before,
                    &self.info.context.options.settings.name,
                ),
                None,
            )?;
        }
        Ok(())
    }

    async fn run_after_hooks(&self) -> Result<()> {
        if let Some(ref hooks) = self.info.context.config.hooks() {
            hook::run(
                &self.info.context,
                hook::collect(
                    hooks.exec(),
                    hook::Phase::After,
                    &self.info.context.options.settings.name,
                ),
                None,
            )?;
        }
        Ok(())
    }

    async fn build(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        _render_options: &RenderOptions,
        output: &mut CompilerOutput,
    ) -> Result<()> {
        // When working with multi-lingual sites the target may not exist yet
        let collation = self.info.context.collation.read().unwrap();
        let path = collation.get_path();
        if !path.exists() {
            fs::create_dir_all(path.as_ref())?;
        }

        let is_incremental = self.info.manifest.is_some();
        if is_incremental {
            info!("Incremental build enabled");
        }

        let manifest_filter = |p: &&Arc<PathBuf>| -> bool {
            if let Some(ref manifest) = self.info.manifest {
                let manifest = manifest.read().unwrap();
                if let Some(target) = collation.get_resource(*p) {
                    match target.as_ref() {
                        Resource::Page { ref target }
                        | Resource::File { ref target } => {
                            let dest = target
                                .get_output(collation.get_path().as_ref());
                            if manifest.exists(p)
                                && !manifest.is_dirty(p, &dest, false)
                            {
                                debug!("[NOOP] {}", p.display());
                                return false;
                            }
                        }
                    }
                }
            }
            true
        };

        let filters = &self.info.sources.filters;
        let plugin_cache = config::plugins_dir()?;
        let plugin_repositories = dirs::repositories_dir()?;
        let plugin_archives = dirs::archives_dir()?;

        // Collect plugins with paths so we can enusre they are not 
        // filtered from the collated files to compile
        let plugin_paths: Vec<PathBuf> = if let Some(ref plugin_cache) = self.info.context.plugins {
            plugin_cache
                .plugins()
                .iter()
                .filter(|(dep, _)| {
                    if let Some(target) = dep.target() {
                        match target {
                            DependencyTarget::File { .. } => true,
                            _ => false 
                        } 
                    } else { false }
                })
                .map(|(dep, _)| {
                    let target = dep.target().as_ref().unwrap();
                    match target {
                        DependencyTarget::File { ref path } => {
                            let base = self.info.context.config.project();
                            // WARN: Assume the plugin logic has already 
                            // WARN: verified the path is available!
                            base.join(path).canonicalize().unwrap()
                        }
                        _ => panic!("Plugin path filter was not configured correctly"),
                    } 
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new() 
        };

        let path_filter = |p: &&Arc<PathBuf>| -> bool {
            // Always allow plugin assets event when path filters are given
            if p.starts_with(&plugin_cache)
                || p.starts_with(&plugin_repositories)
                || p.starts_with(&plugin_archives)
            {
                return true;
            }

            for local_plugin in &plugin_paths {
                if p.starts_with(local_plugin) {
                    return true;
                }
            }

            if let Some(ref filters) = filters {
                for f in filters.iter() {
                    // NOTE: the starts_with() is important so that directory
                    // NOTE: filters will compile everything in the directory
                    if p.starts_with(f) {
                        return true;
                    }
                }
                return false;
            }
            true
        };

        let filter = |p: &&Arc<PathBuf>| -> bool {

            let filtered = path_filter(p);
            if filtered && is_incremental {
                return manifest_filter(p);
            }
            filtered
        };

        self.run_before_hooks().await?;

        compile(&self.info.context, parser, output, filter).await?;

        self.run_after_hooks().await?;

        if is_incremental {
            if let Some(ref manifest) = self.info.manifest {
                debug!("Incremental build update: {}", output.files.len());
                let mut manifest = manifest.write().unwrap();
                manifest.update(&output.files);
            }
        }

        Ok(())
    }

    async fn one(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        file: &PathBuf,
    ) -> Result<()> {
        let _ = run::one(&self.info.context, parser, &file).await?;

        // TODO: update the manifest in single file mode!

        Ok(())
    }
}
