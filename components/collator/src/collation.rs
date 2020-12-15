use std::collections::{hash_map, HashMap, HashSet};
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

    pub fn templates(&self) -> &HashSet<Arc<PathBuf>> {
        // FIXME: support override locale-specific templates!
        &self.fallback.templates
    }

    pub fn get_menu_template_name(&self, name: &str) -> String {
        format!("{}/{}", MENU_TEMPLATE_PREFIX, name)
    }

    pub fn get_menus(&self) -> &HashMap<String, MenuResult> {
        &self.locale.menus
    }

    /// Generate a map of menu identifiers to the URLs
    /// for each page in the menu so templates can iterate
    /// menus.
    pub fn menu_page_href(&self) -> HashMap<&String, Vec<&String>> {
        let mut result: HashMap<&String, Vec<&String>> = HashMap::new();
        for (key, menu) in self.locale.menus.iter() {
            let mut refs = Vec::new();
            menu.pages.iter().for_each(|s| {
                refs.push(s.as_ref());
            });
            result.insert(key, refs);
        }
        result
    }

    pub fn remove_file(&mut self, path: &PathBuf) {
        println!("Collation removing the file {:?}", path);
        /*
        let locale = Arc::make_mut(&mut self.locale);
        println!("Collation removing the file on the locale!!! {:?}", path);
        locale.remove_file(path);
        */
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

    /// Menu definitions.
    pub menus: HashMap<String, MenuResult>,

    // Additional redirects, typically  from pages
    // that have permalinks map the permalink to the
    // computed href but also for books that need to
    // redirect to the first chapter.
    pub(crate) redirects: HashMap<String, String>,

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

    // Custom page specific layouts
    pub(crate) layouts: HashMap<String, Arc<PathBuf>>,

    // Templates in the source tree but outside
    // the `partials` and `layouts` conventions.
    pub(crate) templates: HashSet<Arc<PathBuf>>,

    // The default layout
    //pub(crate) layout: Option<Arc<PathBuf>>,
    pub(crate) links: LinkMap,
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

    fn find_menu(&self, name: &str) -> Option<&MenuResult>;
}

/// Access to the layouts.
pub trait LayoutCollate {
    /// Get the primary layout.
    fn get_layout(&self) -> Option<&Arc<PathBuf>>;

    /// Get all layouts keyed by layout name suitable
    /// for configuring as templates.
    fn layouts(&self) -> &HashMap<String, Arc<PathBuf>>;
}

pub trait LinkCollate {
    fn get_link(&self, key: &String) -> Option<&Arc<PathBuf>>;
    fn get_link_source(&self, key: &PathBuf) -> Option<&Arc<String>>;

    /// Normalize a URL path so that it begins with a leading slash
    /// and is given an `index.html` suffix if it ends with a slash.
    ///
    /// Any fragment identifier should be stripped.
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

        if s.contains('#') {
            let parts: Vec<&str> = s.splitn(2, '#').collect();
            s = parts.get(0).unwrap().to_string();
        }

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
        //println!("Looking for link with key {}", key);

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

    fn find_menu(&self, name: &str) -> Option<&MenuResult> {
        self.locale
            .find_menu(name)
            .or(self.fallback.find_menu(name))
    }
}

impl LayoutCollate for Collation {
    fn get_layout(&self) -> Option<&Arc<PathBuf>> {
        self.locale.get_layout().or(self.fallback.get_layout())
    }

    fn layouts(&self) -> &HashMap<String, Arc<PathBuf>> {
        // TODO: prefer locale layouts?
        self.fallback.layouts()
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

    fn find_menu(&self, name: &str) -> Option<&MenuResult> {
        self.menus.get(name)
    }
}

impl LayoutCollate for CollateInfo {
    fn get_layout(&self) -> Option<&Arc<PathBuf>> {
        self.layouts.get(config::DEFAULT_LAYOUT_NAME)
    }

    fn layouts(&self) -> &HashMap<String, Arc<PathBuf>> {
        &self.layouts
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

    pub fn add_layout(
        &mut self,
        key: String,
        file: Arc<PathBuf>,
    ) -> &mut Arc<PathBuf> {
        self.layouts.entry(key).or_insert(file)
    }

    pub fn add_template(&mut self, file: Arc<PathBuf>) -> bool {
        self.templates.insert(file)
    }

    pub fn get_redirects(&self) -> &HashMap<String, String> {
        &self.redirects
    }

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
        // For files outside the source we need to
        // add to the link map using an alternative base
        // that is stripped then made relative to the source
        // so that links are located correctly.
        _base: Option<&PathBuf>,
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
                //println!("Adding file link for key {}", key.display());
                //println!("Adding file link for href {}", href);

                self.resources.insert(Arc::clone(&key));

                // Ensure link paths are always relative to the source even
                // when using synthetic files outside the source directory
                /*
                let link_key = if let Some(base) = base {
                    let path = options.source.join(key.strip_prefix(base)?);
                    Arc::new(path)
                } else {
                    Arc::clone(&key)
                };
                */

                self.link(Arc::clone(&key), Arc::new(href))?;
            }
            _ => {}
        }

        self.all.insert(key, Resource::new(dest, kind, op));

        Ok(())
    }

    pub fn remove_file(&mut self, path: &PathBuf) {
        println!("CollateInfo removing the file {:?}", path);
        // FIXME: update the link map
        // FIXME: remove from resources
        self.all.remove(path);
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
