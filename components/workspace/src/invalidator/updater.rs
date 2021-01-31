use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use log::{info, warn};

use collections::{CollectionDataBase, CollectionsMap};
use config::{hook::HookConfig, Config, RuntimeOptions, SourceProvider};

use crate::{
    project::Project,
    renderer::{RenderOptions, Renderer},
    Result,
};

use super::{
    utils::{extract_locale, relative_to},
    Invalidation, Kind,
};

pub struct Updater {
    project: Project,

    /// A buffer of page paths, such as `/docs/navigation/index.html`
    /// which map to the file system path that can be used to
    /// resolve the page data.
    ///
    /// This is used so that pages can be dynamically rendered when they
    /// are requested via the web server; rather than ahead-of-time
    /// compiled when changes happen.
    ///
    /// When we JIT compile a page in the buffer it is removed so it is
    /// only compiled once after it has been marked as changed by being
    /// added to this buffer.
    ///
    /// Paths are stored as they are received; when paths come from
    /// file system notifications they are canonical. Before finding
    /// a page in the collation use `relative_to()` to go back to a
    /// path that can resolve a page in the collation.
    ///
    buffer: HashMap<String, PathBuf>,
}

impl Updater {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            buffer: HashMap::new(),
        }
    }

    pub fn config(&self) -> &Config {
        self.project.config()
    }

    pub fn options(&self) -> &RuntimeOptions {
        self.project.options()
    }

    pub fn collections(&self) -> &Arc<RwLock<CollectionsMap>> {
        self.project.collections()
    }

    pub fn renderers(&self) -> &Vec<Renderer> {
        self.project.renderers()
    }

    pub fn has_page_path(&self, href: &str) -> bool {
        self.buffer.contains_key(href)
    }

    pub async fn render(&mut self, href: &str) -> Result<()> {
        if let Some(path) = self.buffer.remove(href) {
            let source = &self.project.options.source;
            let file = relative_to(path, source, source)?;
            self.one(&file).await?;
        }
        Ok(())
    }

    pub(crate) fn update_deletions(
        &mut self,
        paths: &HashSet<PathBuf>,
    ) -> Result<()> {
        let project_path = self.project.config.project().to_path_buf();
        let cwd = std::env::current_dir()?;

        for path in paths {
            // NOTE: cannot use relative_to() when files have been deleted
            // NOTE: because is call canonicalize() which can fail
            let relative = if project_path.is_absolute() {
                path.strip_prefix(&project_path)
                    .unwrap_or(path)
                    .to_path_buf()
            } else {
                path.strip_prefix(&cwd).unwrap_or(path).to_path_buf()
            };

            let (lang, path) = extract_locale(
                &relative,
                self.project.locales.languages().alternate(),
            );
            self.remove_file(&path, lang)?;
        }
        Ok(())
    }

    /// Execute hooks that have changed.
    pub(crate) async fn update_hooks(
        &mut self,
        hooks: &HashSet<(HookConfig, PathBuf)>,
    ) -> Result<()> {
        for (hook, file) in hooks {
            self.project.run_hook(hook, Some(file)).await?;
        }
        Ok(())
    }

    /// Update templates.
    pub(crate) async fn update_templates(
        &mut self,
        templates: &HashSet<PathBuf>,
    ) -> Result<()> {
        for template in templates {
            let name = template.to_string_lossy();
            if template.exists() {
                info!("Render template {}", &name);
                for (parser, renderer) in self.project.iter_mut() {
                    // Re-compile the template
                    parser.load(template)?;

                    let collation =
                        &*renderer.info.context.collation.read().unwrap();
                    let fallback = collation.fallback.read().unwrap();

                    // Update the JIT buffer with all pages!
                    let all_pages = fallback.link_map();
                    self.buffer.extend(all_pages);
                }
            } else {
                info!("Delete template {}", &name);
                for parser in self.project.parsers_mut().iter_mut() {
                    // Remove the template from the parser
                    parser.remove(&name);
                }
            }
        }

        Ok(())
    }

    /// Update partials.
    pub(crate) async fn update_partials(
        &mut self,
        partials: &HashSet<PathBuf>,
    ) -> Result<()> {
        let partials: Vec<(String, &PathBuf)> = partials
            .iter()
            .map(|layout| {
                let name =
                    layout.file_stem().unwrap().to_string_lossy().into_owned();
                (name, layout)
            })
            .collect();

        for (name, partial) in partials {
            if partial.exists() {
                info!("Render partial {}", &name);
                for (parser, renderer) in self.project.iter_mut() {
                    // Re-compile the template
                    parser.add(name.to_string(), partial)?;

                    let collation =
                        &*renderer.info.context.collation.read().unwrap();
                    let fallback = collation.fallback.read().unwrap();

                    // Update the JIT buffer with all pages!
                    let all_pages = fallback.link_map();
                    self.buffer.extend(all_pages);
                }
            } else {
                info!("Delete partial {}", &name);
                for parser in self.project.parsers_mut().iter_mut() {
                    // Remove the partial from the parser
                    parser.remove(&name);
                }
            }
        }

        Ok(())
    }

    /// Update includes.
    pub(crate) async fn update_includes(
        &mut self,
        _includes: &HashSet<PathBuf>,
    ) -> Result<()> {
        for (_parser, renderer) in self.project.iter_mut() {
            let collation = &*renderer.info.context.collation.read().unwrap();
            let fallback = collation.fallback.read().unwrap();

            // Update the JIT buffer with all pages!
            let all_pages = fallback.link_map();
            self.buffer.extend(all_pages);
        }
        Ok(())
    }

    /// Update layouts and render any pages referenced by the layouts.
    pub(crate) async fn update_layouts(
        &mut self,
        layouts: &HashSet<PathBuf>,
    ) -> Result<()> {
        // List of pages to invalidate
        let mut invalidated: HashMap<String, PathBuf> = HashMap::new();

        let layouts: Vec<(String, &PathBuf)> = layouts
            .iter()
            .map(|layout| {
                let name =
                    layout.file_stem().unwrap().to_string_lossy().into_owned();
                (name, layout)
            })
            .collect();

        // TODO: handle new layouts
        // TODO: handle deleted layouts

        for (name, layout) in layouts {
            if layout.exists() {
                info!("Render layout {}", &name);
                for (parser, renderer) in self.project.iter_mut() {
                    // Re-compile the template
                    parser.add(name.to_string(), layout)?;

                    // Collect pages that match the layout name
                    // so they can be rendered
                    let collation =
                        &*renderer.info.context.collation.read().unwrap();
                    let fallback = collation.fallback.read().unwrap();
                    for (file_path, page_lock) in fallback.pages.iter() {
                        let page = page_lock.read().unwrap();
                        if !page.is_standalone() {
                            if let Some(ref layout_name) = page.layout {
                                if &name == layout_name {
                                    if let Some(href) =
                                        collation.get_link_href(file_path)
                                    {
                                        invalidated.insert(
                                            href.to_string(),
                                            file_path.to_path_buf(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                info!("Delete layout {}", &name);
                for (parser, renderer) in self.project.iter_mut() {
                    // Remove the layout from the parser
                    parser.remove(&name);
                    // Remove from the collated data
                    let mut collation =
                        renderer.info.context.collation.write().unwrap();
                    collation.remove_layout(&name);
                }
            }
        }

        self.buffer.extend(invalidated);

        Ok(())
    }

    /// Update collections.
    ///
    /// For now this is very basic and just loads and invalidates the
    /// entire index.
    ///
    /// We don't know which pages should change so we invalidate all
    /// pages.
    ///
    pub(crate) async fn update_collections(
        &mut self,
        collections: &HashSet<(String, PathBuf)>,
        pages: Vec<&PathBuf>,
    ) -> Result<()> {
        let mut db_names = collections
            .iter()
            .map(|(nm, _)| nm.to_string())
            .collect::<HashSet<_>>();

        // Must be canonical becaause page paths are absolute
        let source_path = self.project.options.source.canonicalize()?;

        // This is the relative path version required to get
        // collections base paths correctly.
        let source = self.project.options.source.clone();

        // Store matchers so we can exclude files from
        // the list of pages to invalidate
        let mut matchers = Vec::new();

        // list of pages that were changed so we can reload
        // their data before invalidation of the database collection.
        //
        // Database collections use the existing collated data so
        // we need to manually update pages that have changed so
        // that front matter changes are reflected on pages that
        // list changed pages, for example, a blog index page
        // with a recents query.
        let mut invalidated_pages = Vec::new();

        // Find any collections that might include any of the target pages
        if !pages.is_empty() {
            let collections = self.project.collections.read().unwrap();

            // Find databases that operate on pages
            let pages_databases: Vec<(&String, &CollectionDataBase)> =
                collections
                    .iter()
                    .filter(|(_, v)| {
                        let provider = v.data_provider().source_provider();
                        if let SourceProvider::Pages = provider {
                            true
                        } else {
                            false
                        }
                    })
                    .collect();

            // Find pages that would be included in the database
            // collection and add the db name to the list of
            // databases to be invalidated
            for (name, db) in pages_databases.into_iter() {
                if !db_names.contains(name) {
                    let base_path =
                        if let Some(ref from) = db.data_provider().from() {
                            source_path.join(from)
                        } else {
                            source_path.to_path_buf()
                        };

                    // Store matchers so we can filter the invalidations to exclude
                    // files that should be excluded from the database collection
                    matchers.push((
                        relative_to(&base_path, &source, &source)?,
                        db.data_provider().matcher().clone(),
                    ));

                    for page_path in pages.iter() {
                        invalidated_pages.push(relative_to(
                            page_path,
                            &source_path,
                            &source,
                        )?);

                        // The page must exist in the `from` path for the
                        // pages collection
                        if page_path.starts_with(&base_path) {
                            if let Ok(relative) =
                                page_path.strip_prefix(&base_path)
                            {
                                // Check the page is not excluded from the collection
                                if db.data_provider().matcher().is_empty()
                                    || !db
                                        .data_provider()
                                        .matcher()
                                        .is_excluded(relative)
                                {
                                    // FIXME: reload the page data!

                                    db_names.insert(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        for (_, renderer) in self.project.iter_mut() {
            // Reload the data for invalidated pages
            //
            // NOTE: must execute before we acquire the read locks below
            // NOTE: otherwise we will get a deadlock!
            for page in invalidated_pages.iter() {
                renderer.reload(page)?;
            }
            //

            let collation = &*renderer.info.context.collation.read().unwrap();
            let fallback = collation.fallback.read().unwrap();

            // Rebuild databases for collections that changed
            let mut collections = renderer.info.collections.write().unwrap();
            for db_name in db_names.iter() {
                if let Some(db) = collections.map_mut().get_mut(db_name) {
                    db.build(
                        db_name,
                        &*renderer.info.context.config,
                        &*renderer.info.context.options,
                        &fallback,
                    )
                    .await?;
                }
            }

            // Update the JIT buffer with all pages!
            if matchers.is_empty() {
                let all_pages = fallback.link_map();
                self.buffer.extend(all_pages);
            // Or filter to respect the collections matcher
            } else {
                let all_pages: HashMap<String, PathBuf> = fallback
                    .link_map()
                    .iter()
                    .filter(|(_, page_path)| {
                        for (base_path, matcher) in matchers.iter() {
                            if page_path.starts_with(base_path) {
                                if let Ok(relative) =
                                    page_path.strip_prefix(base_path)
                                {
                                    if matcher.is_excluded(relative) {
                                        return false;
                                    }
                                }
                            }
                        }
                        true
                    })
                    .map(|(k, v)| (k.to_string(), v.to_path_buf()))
                    .collect();

                self.buffer.extend(all_pages);
            }
        }

        Ok(())
    }

    // Remove synthetic pages from the invalidation buffer.
    fn filter_synthetics(&mut self) {
        for (_, renderer) in self.project.iter_mut() {
            let collation = &*renderer.info.context.collation.read().unwrap();
            self.buffer.retain(|_, page_path| {
                if let Some(page_data) = collation.resolve(page_path) {
                    let reader = page_data.read().unwrap();
                    if reader.is_synthetic() {
                        return false;
                    }
                }
                true
            });
        }
    }

    pub async fn invalidate(&mut self, rule: &Invalidation) -> Result<()> {
        // Remove deleted files.
        if !rule.deletions.is_empty() {
            self.update_deletions(&rule.deletions)?;
        }

        // Execute hooks
        if !rule.hooks.is_empty() {
            self.update_hooks(&rule.hooks).await?;
        }

        // Compile templates in the site source tree
        if !rule.templates.is_empty() {
            self.update_templates(&rule.templates).await?;
        }

        // Compile partials
        if !rule.partials.is_empty() {
            self.update_partials(&rule.partials).await?;
        }

        // Invalidate includes
        if !rule.includes.is_empty() {
            self.update_includes(&rule.includes).await?;
        }

        // Compile layouts
        if !rule.layouts.is_empty() {
            self.update_layouts(&rule.layouts).await?;
        }

        // Gather pages so we can test if they should cause
        // a collections invalidation
        let pages: Vec<&PathBuf> = rule
            .actions
            .iter()
            .filter(|action| {
                if let Kind::Page(_) = action {
                    true
                } else {
                    false
                }
            })
            .map(|action| match action {
                Kind::Page(path) => path,
                _ => {
                    panic!("Got unsupported page kind in invalidation updater")
                }
            })
            .collect();

        // Update collections data sources
        if !rule.collections.is_empty() || !pages.is_empty() {
            self.update_collections(&rule.collections, pages).await?;
        }

        // Must remove any synthetic pages from the list of pages to render server-side.
        self.filter_synthetics();

        for action in &rule.actions {
            match action {
                Kind::Page(path) | Kind::File(path) => {
                    // Make the path relative to the project source
                    // as the notify crate gives us an absolute path
                    let source = self.project.options.source.clone();
                    let file = relative_to(path, &source, &source)?;

                    self.one(&file).await?;
                }
            }
        }
        Ok(())
    }

    /// Render a single file using the appropriate locale-specific renderer.
    async fn one(&mut self, file: &PathBuf) -> Result<()> {
        // Raw source files might be localized variants
        // we need to strip the locale identifier from the
        // file path before compiling
        let (lang, file) =
            extract_locale(&file, self.project.locales.languages().alternate());
        let lang: &str = if let Some(ref lang) = lang {
            lang.as_str()
        } else {
            self.config().lang()
        };

        let options = RenderOptions::new_file_lang(
            file,
            lang.to_string(),
            true,
            false,
            false,
        );

        self.project.render(options).await?;

        Ok(())
    }

    /// Helper function to remove a file from the collation.
    fn remove_file(
        &mut self,
        path: &PathBuf,
        mut lang: Option<String>,
    ) -> Result<()> {
        let lang = if let Some(lang) = lang.take() {
            lang
        } else {
            self.project.config().lang().to_string()
        };

        // Find the correct renderer so we access the collation
        // for the language
        if let Some(renderer) = self.project.renderers().iter().find(|r| {
            let collation = r.info.context.collation.read().unwrap();
            let locale = collation.locale.read().unwrap();
            locale.lang == lang
        }) {
            info!("Delete {} -> {}", &lang, path.display());

            // Get the href we can use to get the build product location
            // for deleting from the build directory
            let mut collation =
                renderer.info.context.collation.write().unwrap();

            // Must get the target href before we remove
            // from the collation
            let href = if let Some(href) = collation.get_link_href(path) {
                Some(href.as_ref().to_string())
            } else {
                None
            };

            // Remove from the internal data structure
            collation.remove_file(path, self.project.options());

            // Now try to remove the build product
            if let Some(ref href) = href {
                let build_file = self.project.options().build_target().join(
                    utils::url::to_path_separator(href.trim_start_matches("/")),
                );

                if build_file.exists() {
                    info!("Remove {}", build_file.display());

                    if let Err(e) = fs::remove_file(&build_file) {
                        warn!(
                            "Failed to remove build file {}: {}",
                            build_file.display(),
                            e
                        );
                    }

                    // If we have an `index.html` file then we might
                    // have an empty directory for the parent, let's
                    // try to clean it up too.
                    if let Some(file_name) = build_file.file_name() {
                        if file_name == OsStr::new(config::INDEX_HTML) {
                            if let Some(parent) = build_file.parent() {
                                // The call to remove_dir() will fail if
                                // the directory is not empty
                                let _ = fs::remove_dir(parent);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
