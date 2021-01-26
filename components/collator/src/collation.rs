use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use owning_ref::RwLockReadGuardRef;

use config::indexer::QueryList;
use config::{Config, MenuResult, Page, RuntimeOptions};
use locale::LocaleName;

use crate::{
    resource::{Resource, ResourceKind, ResourceOperation},
    Error, Result,
};

static MENU_TEMPLATE_PREFIX: &str = "@menu";

#[derive(Debug, Default)]
pub struct Collation {
    pub fallback: Arc<RwLock<CollateInfo>>,
    pub locale: Arc<RwLock<CollateInfo>>,
}

impl Collation {
    //fn get_lang(&self) -> &str {
    pub fn get_lang(&self) -> RwLockReadGuardRef<'_, CollateInfo, str> {
        RwLockReadGuardRef::new(self.locale.read().unwrap())
            .map(|rg| rg.get_lang())

        //self.locale.read().unwrap().get_lang()
    }

    // fn get_path(&self) -> &PathBuf {
    pub fn get_path(&self) -> RwLockReadGuardRef<'_, CollateInfo, PathBuf> {
        RwLockReadGuardRef::new(self.locale.read().unwrap())
            .map(|rg| rg.get_path())

        //self.locale.read().unwrap().get_path()
    }

    //fn get_resource(&self, key: &PathBuf) -> Option<&Resource> {
    pub fn get_resource(
        &self,
        key: &PathBuf,
    ) -> Option<RwLockReadGuardRef<'_, CollateInfo, Resource>> {
        self.locale
            .read()
            .unwrap()
            .get_resource(key)
            .map(|_| {
                RwLockReadGuardRef::new(self.locale.read().unwrap())
                    .map(|rg| rg.get_resource(key).unwrap())
            })
            .or({
                self.locale.read().unwrap().get_resource(key).map(|_| {
                    RwLockReadGuardRef::new(self.fallback.read().unwrap())
                        .map(|rg| rg.get_resource(key).unwrap())
                })
            })

        //self.locale
        //.read()
        //.unwrap()
        //.get_resource(key)
        //.or(self.fallback.read().unwrap().get_resource(key))
    }

    //fn resolve(&self, key: &PathBuf) -> Option<&Arc<RwLock<Page>>> {
    pub fn resolve(
        &self,
        key: &PathBuf,
    ) -> Option<RwLockReadGuardRef<'_, CollateInfo, Arc<RwLock<Page>>>> {
        self.locale
            .read()
            .unwrap()
            .resolve(key)
            .map(|_| {
                RwLockReadGuardRef::new(self.locale.read().unwrap())
                    .map(|rg| rg.resolve(key).unwrap())
            })
            .or({
                self.locale.read().unwrap().resolve(key).map(|_| {
                    RwLockReadGuardRef::new(self.fallback.read().unwrap())
                        .map(|rg| rg.resolve(key).unwrap())
                })
            })

        //self.locale.read().unwrap().resolve(key).or(self.fallback.read().unwrap().resolve(key))
    }

    pub fn get_layout(
        &self,
    ) -> Option<RwLockReadGuardRef<'_, CollateInfo, Arc<PathBuf>>> {
        //self.locale.read().unwrap()
        //.get_layout().or(self.fallback.read().unwrap().get_layout())

        self.locale
            .read()
            .unwrap()
            .get_layout()
            .map(|_| {
                RwLockReadGuardRef::new(self.locale.read().unwrap())
                    .map(|rg| rg.get_layout().unwrap())
            })
            .or({
                self.locale.read().unwrap().get_layout().map(|_| {
                    RwLockReadGuardRef::new(self.fallback.read().unwrap())
                        .map(|rg| rg.get_layout().unwrap())
                })
            })
    }

    //fn layouts(&self) -> &HashMap<String, Arc<PathBuf>> {
    pub fn layouts(
        &self,
    ) -> RwLockReadGuardRef<'_, CollateInfo, HashMap<String, Arc<PathBuf>>>
    {
        // TODO: prefer locale layouts?
        //self.fallback.read().unwrap().layouts()

        RwLockReadGuardRef::new(self.fallback.read().unwrap())
            .map(|rg| rg.layouts())
    }

    pub fn remove_layout(&mut self, name: &str) {
        let mut fallback = self.fallback.write().unwrap();
        fallback.layouts.remove(name);
    }

    //fn find_menu(&self, name: &str) -> Option<&MenuResult> {
    pub fn find_menu(
        &self,
        name: &str,
    ) -> Option<RwLockReadGuardRef<'_, CollateInfo, MenuResult>> {
        self.locale
            .read()
            .unwrap()
            .find_menu(name)
            .map(|_| {
                RwLockReadGuardRef::new(self.locale.read().unwrap())
                    .map(|rg| rg.find_menu(name).unwrap())
            })
            .or({
                self.locale.read().unwrap().find_menu(name).map(|_| {
                    RwLockReadGuardRef::new(self.fallback.read().unwrap())
                        .map(|rg| rg.find_menu(name).unwrap())
                })
            })

        //self.locale
        //.read()
        //.unwrap()
        //.find_menu(name)
        //.or(self.fallback.read().unwrap().find_menu(name))
    }

    /*
    pub fn resources(&self) -> Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_> {
    //pub fn resources(&self) -> RwLockReadGuardRef<'_, CollateInfo, Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_>> {
        if self.is_fallback() {
            return self.fallback.read().unwrap().resources();
            //return RwLockReadGuardRef::new(self.fallback.read().unwrap())
                //.map(|rg| rg.resources())
        }

        Box::new(self.locale.read().unwrap().resources.union(&self.fallback.read().unwrap().resources))
    }
    */

    /*
    fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Arc<RwLock<Page>>)> + Send + '_>
    */

    /*
    pub fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Arc<RwLock<Page>>)> + Send + '_>
    {
        if self.is_fallback() {
            return self.fallback.read().unwrap().pages();
        }

        Box::new(self.fallback.read().unwrap().pages.iter().chain(self.locale.read().unwrap().pages.iter()))
    }
    */

    pub fn is_fallback(&self) -> bool {
        self.fallback.read().unwrap().lang == self.locale.read().unwrap().lang
    }

    /*
    //pub fn get_resource(&self, file: &PathBuf) -> Option<&Resource> {
    pub fn get_resource(&self, file: &PathBuf) -> RwLockReadGuardRef<'_, CollateInfo, Resource> {
        RwLockReadGuardRef::new(self.fallback.read().unwrap())
            .map(|rg| rg.all.get(file).unwrap())

        //self.fallback.read().unwrap().all.get(file)
    }
    */

    //pub fn templates(&self) -> &HashSet<Arc<PathBuf>> {
    pub fn templates(
        &self,
    ) -> RwLockReadGuardRef<'_, CollateInfo, HashSet<Arc<PathBuf>>> {
        // FIXME: support override locale-specific templates!
        RwLockReadGuardRef::new(self.fallback.read().unwrap())
            .map(|rg| &rg.templates)

        //&self.fallback.read().unwrap().templates
    }

    //fn get_link(&self, key: &String) -> Option<&Arc<PathBuf>> {
    pub fn get_link_path(
        &self,
        key: &String,
    ) -> Option<RwLockReadGuardRef<'_, CollateInfo, Arc<PathBuf>>> {
        //self.locale.read().unwrap().get_link(key).or(self.fallback.read().unwrap().get_link(key))

        self.locale
            .read()
            .unwrap()
            .get_link_path(key)
            .map(|_| {
                RwLockReadGuardRef::new(self.locale.read().unwrap())
                    .map(|rg| rg.get_link_path(key).unwrap())
            })
            .or({
                self.locale.read().unwrap().get_link_path(key).map(|_| {
                    RwLockReadGuardRef::new(self.fallback.read().unwrap())
                        .map(|rg| rg.get_link_path(key).unwrap())
                })
            })
    }

    //fn get_link_href(&self, key: &PathBuf) -> Option<&Arc<String>> {
    pub fn get_link_href(
        &self,
        key: &PathBuf,
    ) -> Option<RwLockReadGuardRef<'_, CollateInfo, Arc<String>>> {
        //self.locale
        //.read()
        //.unwrap()
        //.get_link_href(key)
        //.or(self.fallback.read().unwrap().get_link_href(key))

        self.locale
            .read()
            .unwrap()
            .get_link_href(key)
            .map(|_| {
                RwLockReadGuardRef::new(self.locale.read().unwrap())
                    .map(|rg| rg.get_link_href(key).unwrap())
            })
            .or({
                self.locale.read().unwrap().get_link_href(key).map(|_| {
                    RwLockReadGuardRef::new(self.fallback.read().unwrap())
                        .map(|rg| rg.get_link_href(key).unwrap())
                })
            })
    }

    pub fn find_link(&self, href: &str) -> Option<PathBuf> {
        self.locale.read().unwrap().find_link(href).or(self
            .fallback
            .read()
            .unwrap()
            .find_link(href))
    }

    pub fn normalize<S: AsRef<str>>(&self, s: S) -> String {
        let fallback = self.fallback.read().unwrap();
        fallback.normalize(s)
    }

    pub fn get_menu_template_name(&self, name: &str) -> String {
        format!("{}/{}", MENU_TEMPLATE_PREFIX, name)
    }

    //pub fn get_menus(&self) -> &HashMap<String, MenuResult> {
    pub fn get_menus(
        &self,
    ) -> RwLockReadGuardRef<'_, CollateInfo, HashMap<String, MenuResult>> {
        RwLockReadGuardRef::new(self.locale.read().unwrap()).map(|rg| &rg.menus)

        //&self.locale.read().unwrap().menus
    }

    /// Generate a map of menu identifiers to the URLs
    /// for each page in the menu so templates can iterate
    /// menus.
    /*
    pub fn menu_page_href(&self) -> HashMap<&String, Vec<&String>> {
        let mut result: HashMap<&String, Vec<&String>> = HashMap::new();
        for (key, menu) in self.locale.read().unwrap().menus.iter() {
            let mut refs = Vec::new();
            menu.pages.iter().for_each(|s| {
                refs.push(s.as_ref());
            });
            result.insert(key, refs);
        }
        result
    }
    */

    // FIXME/WIP: how to use pointers to the strings like we did
    // FIXME/WIP: before the migration to RwLock<CollateInfo>? ^^^^
    pub fn menu_page_href(&self) -> HashMap<String, Vec<String>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for (key, menu) in self.locale.read().unwrap().menus.iter() {
            //let mut refs = Vec::new();
            //menu.pages.iter().for_each(|s| {
            //refs.push(s.as_ref());
            //});
            let refs =
                menu.pages.iter().map(|s| s.as_ref().to_string()).collect();
            result.insert(key.to_owned(), refs);
        }
        result
    }

    /// Remove a file from the locale associated with this collation.
    pub fn remove_file(&mut self, path: &PathBuf, options: &RuntimeOptions) {
        let mut locale = self.locale.write().unwrap();
        locale.remove_file(path, options);
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
    pub resources: HashSet<Arc<PathBuf>>,

    /// Lookup table for page data resolved by locale identifier and source path.
    pub pages: HashMap<Arc<PathBuf>, Arc<RwLock<Page>>>,

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

impl LinkMap {
    /// Normalize a URL path so that it begins with a leading slash
    /// and is given an `index.html` suffix if it ends with a slash.
    ///
    /// Any fragment identifier should be stripped.
    pub fn normalize<S: AsRef<str>>(&self, s: S) -> String {
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

    pub fn get_link_path(&self, key: &String) -> Option<&Arc<PathBuf>> {
        self.reverse.get(key)
    }

    pub fn get_link_href(&self, key: &PathBuf) -> Option<&Arc<String>> {
        self.sources.get(key)
    }

    pub fn find_link(&self, href: &str) -> Option<PathBuf> {
        let mut key = self.normalize(href);
        //println!("Looking for link with key {}", key);

        if let Some(path) = self.get_link_path(&key) {
            return Some(path.to_path_buf());
        } else {
            // Sometimes we have directory references without a trailing slash
            // so try again with an index page
            key.push('/');
            key.push_str(config::INDEX_HTML);
            if let Some(path) = self.get_link_path(&key) {
                return Some(path.to_path_buf());
            }
        }
        None
    }

    pub fn remove(&mut self, path: &PathBuf) -> bool {
        let href = self.sources.remove(path);
        let mut removed = href.is_some();
        if let Some(href) = href {
            removed = removed && self.reverse.remove(&*href).is_some();
        }
        removed
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

    pub fn get_lang(&self) -> &str {
        &self.lang
    }

    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get_resource(&self, key: &PathBuf) -> Option<&Resource> {
        self.all.get(key)
    }

    pub fn resolve(&self, key: &PathBuf) -> Option<&Arc<RwLock<Page>>> {
        self.pages.get(key)
    }

    pub fn find_menu(&self, name: &str) -> Option<&MenuResult> {
        self.menus.get(name)
    }

    pub fn resources(
        &self,
    ) -> Box<dyn Iterator<Item = &Arc<PathBuf>> + Send + '_> {
        Box::new(self.resources.iter())
    }

    pub fn pages(
        &self,
    ) -> Box<dyn Iterator<Item = (&Arc<PathBuf>, &Arc<RwLock<Page>>)> + Send + '_>
    {
        Box::new(self.pages.iter())
    }

    pub fn get_layout(&self) -> Option<&Arc<PathBuf>> {
        self.layouts.get(config::DEFAULT_LAYOUT_NAME)
    }

    pub fn layouts(&self) -> &HashMap<String, Arc<PathBuf>> {
        &self.layouts
    }

    pub fn add_layout(
        &mut self,
        key: String,
        file: Arc<PathBuf>,
    ) -> &mut Arc<PathBuf> {
        self.layouts.entry(key).or_insert(file)
    }

    pub fn get_link_path(&self, key: &String) -> Option<&Arc<PathBuf>> {
        self.links.get_link_path(key)
    }

    pub fn get_link_href(&self, key: &PathBuf) -> Option<&Arc<String>> {
        self.links.get_link_href(key)
    }

    /// Get a clone of the link map reverse lookup.
    pub fn link_map(&self) -> HashMap<String, PathBuf> {
        self.links
            .reverse
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_path_buf()))
            .collect()
    }

    pub fn normalize<S: AsRef<str>>(&self, s: S) -> String {
        self.links.normalize(s)
    }

    pub fn find_link(&self, href: &str) -> Option<PathBuf> {
        self.links.find_link(href)
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

    pub fn get_page(
        &self,
        key: &PathBuf,
    ) -> Option<&Arc<RwLock<Page>>> {
        self.pages.get(key)
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

        let kind = self.get_file_kind(&*key, options);
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

    pub fn remove_file(
        &mut self,
        path: &PathBuf,
        options: &RuntimeOptions,
    ) -> bool {
        let mut removed = self.all.remove(path).is_some();
        let kind = self.get_file_kind(path, options);
        match kind {
            ResourceKind::File | ResourceKind::Asset => {
                removed = removed && self.resources.remove(path);
                removed = removed && self.links.remove(path);
            }
            _ => {}
        }
        removed
    }

    fn get_file_kind(
        &self,
        key: &PathBuf,
        options: &RuntimeOptions,
    ) -> ResourceKind {
        let mut kind = ResourceKind::File;
        if key.starts_with(options.assets_path()) {
            kind = ResourceKind::Asset;
        } else if key.starts_with(options.partials_path()) {
            kind = ResourceKind::Partial;
        } else if key.starts_with(options.includes_path()) {
            kind = ResourceKind::Include;
        } else if key.starts_with(options.locales_path()) {
            kind = ResourceKind::Locale;
        } else if key.starts_with(options.collections_path()) {
            kind = ResourceKind::Collection;
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
