use std::fs::{self, File};

use log::info;
use url::Url;

use human_bytes::human_bytes;

use collator::{Collate, LinkCollate};
use compiler::{parser::Parser, CompileInfo, Compiler, ParseData};
use config::sitemap::{SiteMapEntry, SiteMapFile, SiteMapIndex};
use locale::Locales;
use search::{
    compile as compile_index, intermediate, Index, IntermediateEntry,
};

use crate::Result;

#[derive(Debug)]
pub struct RenderResult {
    pub sitemap: Option<Url>,
}

#[derive(Debug)]
pub struct Renderer {
    pub info: CompileInfo,
}

impl Renderer {
    /// Render a locale for a project.
    pub async fn render(&self, locales: &Locales) -> Result<RenderResult> {
        let (_, _, parse_list) = self.build(locales).await?;
        Ok(RenderResult {
            sitemap: self.finish(parse_list)?,
        })
    }

    fn finish<'a>(&self, parse_list: Vec<ParseData>) -> Result<Option<Url>> {
        self.create_search_indices(&parse_list)?;
        Ok(self.create_site_map(&parse_list)?)
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

    async fn build<'a>(
        &'a self,
        locales: &'a Locales,
    ) -> std::result::Result<
        (Compiler<'_>, Parser<'_>, Vec<ParseData>),
        compiler::Error,
    > {
        // When working with multi-lingual sites the target may not exist yet
        let path = self.info.context.collation.get_path();
        if !path.exists() {
            fs::create_dir_all(path)?;
        }

        let parser = Parser::new(&self.info.context, &locales)?;
        let builder = Compiler::new(&self.info.context);
        let parse_list = builder.all(&parser, &self.info.sources).await?;
        Ok((builder, parser, parse_list))
    }
}
