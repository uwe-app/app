use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use config::indexer::QueryList;
use config::{LocaleName, Page, RuntimeOptions};

use super::manifest::Manifest;

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

#[derive(Debug, Clone)]
pub enum ResourceKind {
    /// A directory encountered whilst walking the tree.
    Dir,
    /// The default type indicates we don't know much about this resource.
    File,
    /// The type of file that renders to an output page.
    Page,
    /// An asset file is typically located in the `assets` folder and
    /// is primarily used for the site layout: images, fonts, styles etc.
    Asset,
    /// A locale resource, typically .ftl files in the `locales` folder.
    Locale,
    /// A partial file provides part of a template render; normally
    /// located in the `partials` directory but can also come from
    /// other locations.
    Partial,
    /// Include files are documents included by pages; they normally
    /// reside in the `includes` directory and are typically used for
    /// code samples etc.
    Include,
    /// This file is part of a data source directory.
    DataSource,
}

impl Default for ResourceKind {
    fn default() -> Self {
        ResourceKind::File
    }
}

/// The compiler uses this as the action to perform
/// with the input source file.
#[derive(Debug, Clone)]
pub enum ResourceOperation {
    // Do nothing, used for the Dir kind primarily.
    Noop,
    // Render a file as a page template
    Render,
    // Copy a file to the build target
    Copy,
    // Create a symbolic link to the source file
    Link,
}

impl Default for ResourceOperation {
    fn default() -> Self {
        ResourceOperation::Copy
    }
}

#[derive(Debug, Default, Clone)]
pub struct ResourceTarget {
    pub destination: PathBuf,
    pub operation: ResourceOperation,
    pub kind: ResourceKind,
}

#[derive(Debug, Clone)]
pub enum Resource {
    Page { target: ResourceTarget },
    File { target: ResourceTarget },
}

impl Resource {
    pub fn new(
        destination: PathBuf,
        kind: ResourceKind,
        op: ResourceOperation,
    ) -> Self {
        let target = ResourceTarget {
            kind,
            destination,
            operation: op,
        };
        Resource::File { target }
    }

    pub fn new_page(destination: PathBuf) -> Self {
        let kind = ResourceKind::Page;
        let target = ResourceTarget {
            kind,
            destination,
            operation: ResourceOperation::Render,
        };
        Resource::Page { target }
    }

    pub fn set_operation(&mut self, operation: ResourceOperation) {
        match self {
            Self::Page { ref mut target } | Self::File { ref mut target } => {
                target.operation = operation;
            }
        }
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
        self.layouts.get(key).or(
            { if let Some(ref layout) = self.layout { Some(layout); } None }
        )
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

    pub fn remove_page(
        &mut self,
        p: &PathBuf,
        options: &RuntimeOptions,
    ) -> Option<Page> {
        //if let Some(pos) = self.resources.iter().position(|x| &**x == p) {
            //self.resources.remove(pos);
        //}
        self.resources.remove(p);
        self.pages.remove(p)
    }
}
