use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use config::indexer::QueryList;
use config::{Config, FileInfo, FileOptions, Page, RuntimeOptions};
use locale::LocaleName;

use crate::manifest::Manifest;
use crate::resource::*;
use crate::Result;

fn get_layout(l: &PathBuf) -> (String, PathBuf) {
    let layout = l.to_path_buf();
    let name = layout.to_string_lossy().into_owned();
    (name, layout)
}

#[derive(Debug, Default)]
pub struct Collation {
    pub fallback: Arc<CollateInfo>,
    pub locale: Arc<CollateInfo>,
}

impl Collation {
    pub fn is_fallback(&self) -> bool {
        self.fallback.lang == self.locale.lang
    }
}

#[derive(Debug, Default, Clone)]
pub struct CollateInfo {
    /// The language for this collation.
    pub lang: LocaleName,

    /// The target output directory for this collation.
    pub path: PathBuf,

    /// All the resources resulting from a collation.
    pub(crate) all: HashMap<Arc<PathBuf>, Resource>,

    /// Lookup table for all the resources that should
    /// be processed by the compiler.
    pub(crate) resources: HashSet<Arc<PathBuf>>,

    /// Lookup table for page data resolved by locale identifier and source path.
    pub(crate) pages: HashMap<Arc<PathBuf>, Page>,

    // Pages that have permalinks map the
    // permalink to the computed href so that
    // we can configure redirects for permalinks.
    pub permalinks: HashMap<String, String>,

    // Pages located for feed configurations.
    //
    // The hash map key is the key for the feed congfiguration
    // and each entry is a path that can be used to
    // locate the page data in `pages`.
    pub feeds: HashMap<String, Vec<Arc<PathBuf>>>,

    // Store queries for expansion later
    pub queries: Vec<(QueryList, Arc<PathBuf>)>,

    // List of series
    pub(crate) series: HashMap<String, Vec<Arc<PathBuf>>>,

    // Custom page specific layouts
    pub(crate) layouts: HashMap<Arc<PathBuf>, PathBuf>,
    // The default layout
    pub(crate) layout: Option<Arc<PathBuf>>,

    // TODO: books too!
    pub(crate) links: LinkMap,

    // Manifest for incremental builds
    pub manifest: Option<Manifest>,
}

#[derive(Debug, Default, Clone)]
pub struct LinkMap {
    pub(crate) sources: HashMap<Arc<PathBuf>, Arc<String>>,
    pub(crate) reverse: HashMap<Arc<String>, Arc<PathBuf>>,
}

/// General access to collated data.
pub trait Collate {
    fn get_lang(&self) -> &str;
    fn get_path(&self) -> &PathBuf;
    fn get_resource(&self, key: &PathBuf) -> Option<&Resource>;
    fn resolve(&self, key: &PathBuf) -> Option<&Page>;
    fn resources(&self) -> Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_>;
    fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Page)> + Send + '_>;
}

/// Access to the collated series.
pub trait SeriesCollate {
    fn get_series(&self, key: &str) -> Option<&Vec<Arc<PathBuf>>>;
}

/// Access to the layouts.
pub trait LayoutCollate {
    /// Get the primary layout.
    fn get_layout(&self) -> Option<Arc<PathBuf>>;

    /// Get all layouts keyed by layout name suitable
    /// for configuring as templates.
    fn layouts(&self) -> HashMap<String, PathBuf>;

    /// Attempt to find a layout for a file path searching
    /// custom layouts and falling back to the default layout
    /// if no custom layout was found for the key.
    fn find_layout(&self, key: &PathBuf) -> Option<&PathBuf>;
}

pub trait LinkCollate {
    fn get_link(&self, key: &String) -> Option<&Arc<PathBuf>>;
    fn get_link_source(&self, key: &PathBuf) -> Option<&Arc<String>>;
}

impl LinkCollate for LinkMap {
    fn get_link(&self, key: &String) -> Option<&Arc<PathBuf>> {
        self.reverse.get(key)
    }

    fn get_link_source(&self, key: &PathBuf) -> Option<&Arc<String>> {
        self.sources.get(key)
    }
}

impl Collate for Collation {
    fn get_lang(&self) -> &str {
        self.locale.get_lang()
    }

    fn get_path(&self) -> &PathBuf {
        self.locale.get_path()
    }

    fn get_resource(&self, key: &PathBuf) -> Option<&Resource> {
        self.locale
            .get_resource(key)
            .or(self.fallback.get_resource(key))
    }

    fn resolve(&self, key: &PathBuf) -> Option<&Page> {
        self.locale.resolve(key).or(self.fallback.resolve(key))
    }

    fn resources(&self) -> Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_> {
        if self.is_fallback() {
            return self.fallback.resources();
        }

        Box::new(self.locale.resources.union(&self.fallback.resources))
    }

    fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Page)> + Send + '_> {
        if self.is_fallback() {
            return self.fallback.pages();
        }

        Box::new(self.fallback.pages.iter().chain(self.locale.pages.iter()))
    }
}

impl SeriesCollate for Collation {
    fn get_series(&self, key: &str) -> Option<&Vec<Arc<PathBuf>>> {
        self.locale
            .get_series(key)
            .or(self.fallback.get_series(key))
    }
}

impl LayoutCollate for Collation {
    fn get_layout(&self) -> Option<Arc<PathBuf>> {
        self.locale.get_layout().or(self.fallback.get_layout())
    }

    fn layouts(&self) -> HashMap<String, PathBuf> {
        // TODO: prefer locale layouts?
        self.fallback.layouts()
    }

    fn find_layout(&self, key: &PathBuf) -> Option<&PathBuf> {
        // TODO: prefer locale layouts?
        self.fallback.find_layout(key)
    }
}

impl LinkCollate for Collation {
    fn get_link(&self, key: &String) -> Option<&Arc<PathBuf>> {
        self.locale.get_link(key).or(self.fallback.get_link(key))
    }

    fn get_link_source(&self, key: &PathBuf) -> Option<&Arc<String>> {
        self.locale
            .get_link_source(key)
            .or(self.fallback.get_link_source(key))
    }
}

impl Collate for CollateInfo {
    fn get_lang(&self) -> &str {
        &self.lang
    }

    fn get_path(&self) -> &PathBuf {
        &self.path
    }

    fn get_resource(&self, key: &PathBuf) -> Option<&Resource> {
        self.all.get(key)
    }

    fn resolve(&self, key: &PathBuf) -> Option<&Page> {
        self.pages.get(key)
    }

    fn resources(&self) -> Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_> {
        Box::new(self.resources.iter())
    }

    fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Page)> + Send + '_> {
        Box::new(self.pages.iter())
    }
}

impl SeriesCollate for CollateInfo {
    fn get_series(&self, key: &str) -> Option<&Vec<Arc<PathBuf>>> {
        self.series.get(key)
    }
}

impl LayoutCollate for CollateInfo {
    fn get_layout(&self) -> Option<Arc<PathBuf>> {
        self.layout.clone()
    }

    fn layouts(&self) -> HashMap<String, PathBuf> {
        let mut map = HashMap::new();
        if let Some(ref layout) = self.get_layout() {
            let (name, path) = get_layout(&layout.to_path_buf());
            map.insert(name, path);
        }

        for (_, layout) in self.layouts.iter() {
            let (name, path) = get_layout(layout);
            map.insert(name, path);
        }

        map
    }

    fn find_layout(&self, key: &PathBuf) -> Option<&PathBuf> {
        if let Some(ref layout) = self.layouts.get(key) {
            return Some(layout);
        }
        if let Some(ref layout) = self.layout {
            return Some(layout);
        }
        None
    }
}

impl LinkCollate for CollateInfo {
    fn get_link(&self, key: &String) -> Option<&Arc<PathBuf>> {
        self.links.get_link(key)
    }

    fn get_link_source(&self, key: &PathBuf) -> Option<&Arc<String>> {
        self.links.get_link_source(key)
    }
}

impl CollateInfo {
    pub fn new(lang: String, path: PathBuf) -> Self {
        Self {
            lang,
            path,
            ..Default::default()
        }
    }

    pub fn get_pages(&self) -> &HashMap<Arc<PathBuf>, Page> {
        &self.pages
    }

    pub fn get_page_mut(&mut self, key: &PathBuf) -> Option<&mut Page> {
        self.pages.get_mut(key)
    }

    pub fn remove_page(&mut self, p: &PathBuf) -> Option<Page> {
        self.resources.remove(p);
        self.pages.remove(p)
    }

    /// Inherit page data from a fallback locale.
    pub fn inherit(
        &mut self,
        config: &Config,
        options: &RuntimeOptions,
        fallback: &mut CollateInfo,
    ) -> Result<()> {
        let mut updated: HashMap<Arc<PathBuf>, Page> = HashMap::new();
        for (path, page) in self.pages.iter_mut() {
            let use_fallback =
                page.fallback.is_some() && page.fallback.unwrap();
            let mut fallback_page = fallback.pages.get_mut(path);
            if let Some(ref mut fallback_page) = fallback_page {
                let file_context = fallback_page.file.as_ref().unwrap();
                let source = file_context.source.clone();

                let mut sub_page = fallback_page.clone();

                let template = if use_fallback {
                    sub_page.file.as_ref().unwrap().template.clone()
                } else {
                    page.file.as_ref().unwrap().template.clone()
                };

                sub_page.append(page);

                let mut rewrite_index = options.settings.should_rewrite_index();
                // Override with rewrite-index page level setting
                if let Some(val) = sub_page.rewrite_index {
                    rewrite_index = val;
                }

                // Must seal() again so the file paths are correct
                let mut file_info =
                    FileInfo::new(config, options, &source, false);
                let file_opts = FileOptions {
                    rewrite_index,
                    base_href: &options.settings.base_href,
                    ..Default::default()
                };
                let dest = file_info.destination(&file_opts)?;
                sub_page.seal(
                    &dest,
                    config,
                    options,
                    &file_info,
                    Some(template),
                )?;

                updated.insert(path.to_owned(), sub_page);
            } else {
                updated.insert(path.to_owned(), page.to_owned());
            }
        }
        self.pages = updated;
        Ok(())
    }
}
