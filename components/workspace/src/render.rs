use std::path::PathBuf;
use std::fs::{self, File};

use log::info;
use url::Url;

use human_bytes::human_bytes;

use compiler::{BuildContext, Compiler, ParseData, parser::Parser};
use config::sitemap::{SiteMapIndex, SiteMapFile, SiteMapEntry};
use locale::Locales;
use search::{compile as compile_index, intermediate, Index, IntermediateEntry};

use crate::Result;

#[derive(Debug)]
pub struct RenderResult {
    pub sitemap: Option<Url>,
}

#[derive(Debug)]
pub struct Render {
    pub context: BuildContext,
    pub paths: Vec<PathBuf>,
}

impl Render {

    pub async fn render(&self, locales: &Locales) -> Result<RenderResult> {
        let (_, _, parse_list) = self.build(locales).await?;

        let mut result = RenderResult {
            sitemap: self.finish(parse_list)?
        };

        Ok(result)
    }

    fn finish<'a>(&self, parse_list: Vec<ParseData>) -> Result<Option<Url>> {
        self.create_search_indices(&parse_list)?;
        Ok(self.create_site_map(&parse_list)?)
    }

    fn create_search_indices(&self, parse_list: &Vec<ParseData>) -> Result<()> {
        let ctx = &self.context;
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

                        if !search.matcher.filter(url) {
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

    fn create_site_map(&self, parse_list: &Vec<ParseData>) -> Result<Option<Url>> {
        let ctx = &self.context;

        let mut res: Option<Url> = None;
        if let Some(ref sitemap) = ctx.options.settings.sitemap {

            //if ctx.options.settings.robots.is_none() {
                //ctx.options.settings.robots = Some(Default::default());
            //}

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
                        let page = ctx.collation.resolve(&d.file, &ctx.options).unwrap();
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

            //if let Some(ref mut robots) = ctx.options.settings.robots.as_mut() {
                //robots.sitemaps.push(sitemap_url);
            //}

            res = Some(sitemap_url);
        }

        Ok(res)
    }

    /*
    async fn compile(&self) -> Result<()> {
        let mut opts = self.context.options.clone();
        let mut ctx = self.context;

        let locale_map = opts.locales;

        let base_target = opts.target.clone();
        let mut previous_base = base_target.clone();

        for lang in locale_map.map.keys() {
            // When we have multiple languages we need to rewrite paths
            // on each iteration for each specific language
            if locale_map.multi {
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
                    .rewrite(&opts, &lang, &previous_base, &locale_target)?;

                previous_base = locale_target;
            }

            //prepare(&mut ctx)?;
            //let (_, _, parse_list) = build(&mut ctx, &locales).await?;
            //finish(&mut ctx, parse_list)?;
        }

        //write_robots_file(&mut ctx)?;

        //Ok((ctx, locales))

        Ok(())
    }
    */

    async fn build<'a>(
        &'a self,
        //ctx: &'a mut BuildContext,
        locales: &'a Locales,
    ) -> std::result::Result<(Compiler<'_>, Parser<'_>, Vec<ParseData>), compiler::Error> {

        // When working with multi-lingual sites the target may not exist yet
        let target = &self.context.options.target;
        if !target.exists() {
            fs::create_dir_all(target)?;
        }

        let parser = Parser::new(&self.context, &locales)?;
        let builder = Compiler::new(&self.context);
        let parse_list = builder.all(&parser, &self.paths).await?;
        Ok((builder, parser, parse_list))
    }
}
