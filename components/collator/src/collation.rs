use std::collections::{HashMap, HashSet, hash_map};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use config::indexer::QueryList;
use config::{Config, MenuEntry, MenuResult, Page, RuntimeOptions};
use locale::LocaleName;

use crate::{
    resource::{Resource, ResourceKind, ResourceOperation},
    Error, Result,
};

static MENU_TEMPLATE_PREFIX: &str = "@menu";

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

    pub fn get_resource(&self, file: &PathBuf) -> Option<&Resource> {
        self.fallback.all.get(file)
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
    pub(crate) pages: HashMap<Arc<PathBuf>, Arc<RwLock<Page>>>,

    /// Graph of page relationships.
    pub(crate) graph: Graph,

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

    // Map of books files so that we can assign the theme template 
    // and book menu.
    pub(crate) books: HashMap<String, Vec<Arc<PathBuf>>>,

    // List of series
    pub(crate) series: HashMap<String, Vec<Arc<PathBuf>>>,

    // Custom page specific layouts
    pub(crate) layouts: HashMap<Arc<PathBuf>, PathBuf>,
    // The default layout
    pub(crate) layout: Option<Arc<PathBuf>>,

    pub(crate) links: LinkMap,
}

#[derive(Debug, Default, Clone)]
pub struct Graph {
    pub(crate) menus: MenuMap,
}

impl Graph {
    pub fn get_menus(&self) -> &MenuMap {
        &self.menus
    }
}

#[derive(Debug, Default, Clone)]
pub struct MenuMap {
    /// List of pages with menus that need to be compiled.
    pub(crate) sources: HashMap<Arc<MenuEntry>, Vec<Arc<PathBuf>>>,

    /// Compiled results for each menu.
    pub(crate) results: HashMap<Arc<MenuEntry>, Arc<MenuResult>>,

    /// Lookup table by file and menu name so the menu helper
    /// can easily locale the menu results.
    pub(crate) mapping: HashMap<Arc<PathBuf>, HashMap<String, Arc<MenuResult>>>,
}

impl MenuMap {

    pub fn get_menu_template_name(&self, name: &str) -> String {
        format!("{}/{}", MENU_TEMPLATE_PREFIX, name) 
    }

    pub fn results(&self) -> hash_map::Iter<'_, Arc<MenuEntry>, Arc<MenuResult>> {
        self.results.iter() 
    }
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
    fn resolve(&self, key: &PathBuf) -> Option<&Arc<RwLock<Page>>>;
    fn resources(&self) -> Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_>;
    fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Arc<RwLock<Page>>)> + Send + '_>;
    fn get_graph(&self) -> &Graph;

    fn find_menu(&self, path: &PathBuf, name: &str) -> Option<&MenuResult>;
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

    /// Normalize a URL path so that it begins with a leading slash
    /// and is given an `index.html` suffix if it ends with a slash.
    fn normalize<S: AsRef<str>>(&self, s: S) -> String;

    /// Try to find a source file corresponging to a link URL path.
    fn find_link(&self, href: &str) -> Option<PathBuf>;
}

impl LinkCollate for LinkMap {
    fn get_link(&self, key: &String) -> Option<&Arc<PathBuf>> {
        self.reverse.get(key)
    }

    fn get_link_source(&self, key: &PathBuf) -> Option<&Arc<String>> {
        self.sources.get(key)
    }

    fn normalize<S: AsRef<str>>(&self, s: S) -> String {
        let mut s = s.as_ref().to_string();
        if !s.starts_with("/") {
            s = format!("/{}", s);
        }
        // We got a hint with the trailing slash that we should look for an index page
        if s != "/" && s.ends_with("/") {
            s.push_str(config::INDEX_HTML);
        }
        s
    }

    fn find_link(&self, href: &str) -> Option<PathBuf> {
        let mut key = self.normalize(href);
        if let Some(path) = self.get_link(&key) {
            return Some(path.to_path_buf());
        } else {
            // Sometimes we have directory references without a trailing slash
            // so try again with an index page
            key.push('/');
            key.push_str(config::INDEX_HTML);
            if let Some(path) = self.get_link(&key) {
                return Some(path.to_path_buf());
            }
        }
        None
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

    fn resolve(&self, key: &PathBuf) -> Option<&Arc<RwLock<Page>>> {
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
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Arc<RwLock<Page>>)> + Send + '_>
    {
        if self.is_fallback() {
            return self.fallback.pages();
        }

        Box::new(self.fallback.pages.iter().chain(self.locale.pages.iter()))
    }

    fn get_graph(&self) -> &Graph {
        self.locale.get_graph()
    }

    fn find_menu(&self, path: &PathBuf, name: &str) -> Option<&MenuResult> {
        self.locale.find_menu(path, name)
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

    fn normalize<S: AsRef<str>>(&self, s: S) -> String {
        self.locale.normalize(s)
    }

    fn find_link(&self, href: &str) -> Option<PathBuf> {
        self.locale
            .find_link(href)
            .or(self.fallback.find_link(href))
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

    fn resolve(&self, key: &PathBuf) -> Option<&Arc<RwLock<Page>>> {
        self.pages.get(key)
    }

    fn resources(&self) -> Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_> {
        Box::new(self.resources.iter())
    }

    fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Arc<RwLock<Page>>)> + Send + '_>
    {
        Box::new(self.pages.iter())
    }

    fn get_graph(&self) -> &Graph {
        &self.graph
    }

    fn find_menu(&self, path: &PathBuf, name: &str) -> Option<&MenuResult> {
        if let Some(menu) = self.graph.menus.mapping.get(path) {
            if let Some(result) = menu.get(name) {
                return Some(result);
            }
        }
        None
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

    fn normalize<S: AsRef<str>>(&self, s: S) -> String {
        self.links.normalize(s)
    }

    fn find_link(&self, href: &str) -> Option<PathBuf> {
        self.links.find_link(href)
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

    pub fn get_graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }

    //pub fn get_books_mut(&mut self) -> &mut HashMap<String, Vec<Arc<PathBuf>>> {
        //&mut self.books
    //}

    /// Create a page in this collation.
    ///
    /// The `key` is the file system path to the source file that
    /// generates this entry and maps to an output file.
    ///
    /// The `dest` is the link href path; so for the input file
    /// of `site/faq.md` the `dest` will be `faq/index.html` asssuming
    /// that the `rewrite_index` setting is on.
    pub fn add_page(
        &mut self,
        key: &Arc<PathBuf>,
        dest: PathBuf,
        page_info: Arc<RwLock<Page>>,
    ) {
        let mut resource = Resource::new_page(dest);
        if let Some(ref render) = page_info.read().unwrap().render {
            if !render {
                resource.set_operation(ResourceOperation::Copy);
            }
        }

        self.all.insert(Arc::clone(key), resource);
        self.resources.insert(Arc::clone(key));
        self.pages.entry(Arc::clone(key)).or_insert(page_info);
    }

    pub fn link(
        &mut self,
        source: Arc<PathBuf>,
        href: Arc<String>,
    ) -> Result<()> {
        if let Some(existing) = self.links.reverse.get(&href) {
            return Err(Error::LinkCollision(
                href.to_string(),
                existing.to_path_buf(),
                source.to_path_buf(),
            ));
        }

        //println!("Link href {:?}", &href);
        self.links
            .reverse
            .entry(Arc::clone(&href))
            .or_insert(Arc::clone(&source));
        self.links.sources.entry(source).or_insert(href);
        Ok(())
    }

    pub fn get_pages(&self) -> &HashMap<Arc<PathBuf>, Arc<RwLock<Page>>> {
        &self.pages
    }

    pub fn get_page_mut(
        &mut self,
        key: &PathBuf,
    ) -> Option<&mut Arc<RwLock<Page>>> {
        self.pages.get_mut(key)
    }

    pub fn remove_page(&mut self, p: &PathBuf) -> Option<Arc<RwLock<Page>>> {
        self.all.remove(p);
        self.resources.remove(p);
        self.pages.remove(p)
    }

    pub fn add_file(
        &mut self,
        options: &RuntimeOptions,
        key: Arc<PathBuf>,
        dest: PathBuf,
        href: String,
    ) -> Result<()> {
        // Set up the default resource operation
        let mut op = if options.settings.is_release() {
            ResourceOperation::Copy
        } else {
            ResourceOperation::Link
        };

        // Allow the profile settings to control the resource operation
        if let Some(ref resources) = options.settings.resources {
            if resources.ignore.matcher.matches(&href) {
                op = ResourceOperation::Noop;
            } else if resources.symlink.matcher.matches(&href) {
                op = ResourceOperation::Link;
            } else if resources.copy.matcher.matches(&href) {
                op = ResourceOperation::Copy;
            }
        }

        let kind = self.get_file_kind(&key, options);
        match kind {
            ResourceKind::File | ResourceKind::Asset => {
                self.resources.insert(Arc::clone(&key));
                self.link(Arc::clone(&key), Arc::new(href))?;
            }
            _ => {}
        }

        self.all.insert(key, Resource::new(dest, kind, op));

        Ok(())
    }

    fn get_file_kind(
        &self,
        key: &Arc<PathBuf>,
        options: &RuntimeOptions,
    ) -> ResourceKind {
        let mut kind = ResourceKind::File;
        if key.starts_with(options.get_assets_path()) {
            kind = ResourceKind::Asset;
        } else if key.starts_with(options.get_partials_path()) {
            kind = ResourceKind::Partial;
        } else if key.starts_with(options.get_includes_path()) {
            kind = ResourceKind::Include;
        } else if key.starts_with(options.get_locales()) {
            kind = ResourceKind::Locale;
        } else if key.starts_with(options.get_data_sources_path()) {
            kind = ResourceKind::DataSource;
        }
        kind
    }

    /// Inherit page data from a fallback locale.
    pub fn inherit(
        &mut self,
        config: &Config,
        options: &RuntimeOptions,
        fallback: &mut CollateInfo,
    ) -> Result<()> {
        let mut updated: HashMap<Arc<PathBuf>, Arc<RwLock<Page>>> =
            HashMap::new();
        for (path, raw_page) in self.pages.iter_mut() {
            let mut page = raw_page.write().unwrap();
            let use_fallback =
                page.fallback.is_some() && page.fallback.unwrap();
            let fallback_page = fallback.pages.get(path);
            if let Some(ref fallback_page) = fallback_page {
                let fallback_page = fallback_page.read().unwrap();

                let file_context = fallback_page.file.as_ref().unwrap();
                let source = file_context.source.clone();

                let mut sub_page = fallback_page.clone();

                let template = if use_fallback {
                    sub_page.file.as_ref().unwrap().template.clone()
                } else {
                    page.file.as_ref().unwrap().template.clone()
                };

                // FIXME: !!!

                sub_page.append(&mut page);

                let mut rewrite_index = options.settings.should_rewrite_index();
                // Override with rewrite-index page level setting
                if let Some(val) = sub_page.rewrite_index {
                    rewrite_index = val;
                }

                let dest = options
                    .destination()
                    .rewrite_index(rewrite_index)
                    .build(&source)?;

                sub_page.seal(
                    config,
                    options,
                    &source,
                    &dest,
                    Some(template),
                )?;

                updated
                    .insert(path.to_owned(), Arc::new(RwLock::new(sub_page)));
            } else {
                updated.insert(path.to_owned(), raw_page.to_owned());
            }
        }
        self.pages = updated;
        Ok(())
    }
}
