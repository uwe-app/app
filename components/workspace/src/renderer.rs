use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use log::{debug, info};
use url::Url;

use human_bytes::human_bytes;

use collator::{resource::Resource, Collate, LinkCollate};
use compiler::{
    compile,
    parser::Parser, run, BuildContext, CompilerOutput, ParseData,
};
use config::sitemap::{SiteMapEntry, SiteMapFile, SiteMapIndex};
use locale::{LocaleName, Locales};
use search::{
    compile as compile_index, intermediate, Index, IntermediateEntry,
};

use crate::{hook, manifest::Manifest, Result};

#[derive(Clone)]
pub enum RenderFilter {
    /// Render every locale.
    All,
    /// Render a single locale.
    One(LocaleName),
}

#[derive(Clone)]
pub enum RenderType {
    /// Render everything for this locale.
    All,
    /// Render a single file.
    File(PathBuf),
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
        render_type: RenderType,
    ) -> Result<RenderResult> {
        let mut output: CompilerOutput = Default::default();

        match render_type {
            RenderType::All => {
                self.build(parser, &mut output).await?;
            }
            RenderType::File(ref path) => {
                self.one(parser, path).await?;
            }
        }

        Ok(RenderResult {
            sitemap: self.finish(output)?,
        })
    }

    fn finish(&self, output: CompilerOutput) -> Result<Option<Url>> {
        self.create_search_indices(&output.data)?;
        Ok(self.create_site_map(&output.data)?)
    }

    fn create_search_indices(&self, parse_list: &Vec<ParseData>) -> Result<()> {
        let ctx = &self.info.context;
        let include_index = ctx.options.settings.should_include_index();
        if let Some(ref search) = ctx.config.search {
            for (_id, search) in search.items.iter() {
                let mut intermediates: Vec<IntermediateEntry> = Vec::new();
                info!("Prepare search index ({})", parse_list.len());
                for parse_data in parse_list {
                    if let Some(ref extraction) = parse_data.extract {
                        let href =
                            ctx.collation.get_link_source(&parse_data.file);

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
                    search.get_output_path(ctx.collation.get_path());
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

        let mut res: Option<Url> = None;
        if let Some(ref sitemap) = ctx.options.settings.sitemap {
            // How many entries per chunk window?
            let entries = sitemap.entries.as_ref().unwrap();

            // Base canonical URL
            let base = ctx.options.get_canonical_url(
                &ctx.config,
                Some(self.info.context.collation.get_lang()),
            )?;

            // Create the top-level index of all sitemaps
            let folder = sitemap.name.as_ref().unwrap().to_string();
            let mut idx = SiteMapIndex::new(base.clone(), folder.clone());

            let base_folder = ctx.collation.get_path().join(&folder);

            if !base_folder.exists() {
                fs::create_dir_all(&base_folder)?;
            }

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
                    .filter(|d| !d.file.ends_with("404.html"))
                    .map(|d| {
                        // Get the href to use to build the location
                        let href =
                            ctx.collation.get_link_source(&d.file).unwrap();
                        // Get the last modification data from the page
                        let page = ctx.collation.resolve(&d.file).unwrap();
                        let page = &*page.read().unwrap();
                        // Generate the absolute location
                        let location = base.join(href).unwrap();
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

    async fn run_before_hooks(&self) -> Result<()> {
        if let Some(ref hooks) = self.info.context.config.hook {
            hook::run(
                Arc::clone(&self.info.context),
                hook::collect(
                    hooks.map.clone(),
                    hook::Phase::Before,
                    &self.info.context.options.settings.name,
                ),
            )?;
        }
        Ok(())
    }

    async fn run_after_hooks(&self) -> Result<()> {
        if let Some(ref hooks) = self.info.context.config.hook {
            hook::run(
                Arc::clone(&self.info.context),
                hook::collect(
                    hooks.map.clone(),
                    hook::Phase::After,
                    &self.info.context.options.settings.name,
                ),
            )?;
        }
        Ok(())
    }

    async fn build(
        &self,
        parser: &Box<impl Parser + Send + Sync + ?Sized>,
        output: &mut CompilerOutput,
    ) -> Result<()> {
        // When working with multi-lingual sites the target may not exist yet
        let path = self.info.context.collation.get_path();
        if !path.exists() {
            fs::create_dir_all(path)?;
        }

        let is_incremental = self.info.manifest.is_some();
        if is_incremental {
            info!("Incremental build enabled");
        }

        let manifest_filter = |p: &&Arc<PathBuf>| -> bool {
            if let Some(ref manifest) = self.info.manifest {
                let manifest = manifest.read().unwrap();
                if let Some(ref resource) =
                    self.info.context.collation.get_resource(*p)
                {
                    match resource {
                        Resource::Page { ref target }
                        | Resource::File { ref target } => {
                            let dest = target.get_output(
                                self.info.context.collation.get_path(),
                            );
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
        let path_filter = |p: &&Arc<PathBuf>| -> bool {
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
