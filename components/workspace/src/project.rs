use std::fs;
use std::ffi::OsStr;
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use log::{debug, info, warn};

use futures::TryFutureExt;
use url::Url;

use collator::{
    self, menu, CollateInfo, CollateRequest, CollateResult, Collation,
};
use compiler::{parser, parser::Parser, BuildContext};

use config::{
    hook::HookConfig, plugin_cache::PluginCache, profile::Profiles,
    syntax::SyntaxConfig, Config, ProfileSettings, RedirectConfig,
    RuntimeOptions,
};

use collections::{synthetic, DataSourceMap, QueryCache};

use locale::Locales;

use crate::{
    manifest::Manifest,
    plugins,
    renderer::{CompilerInput, RenderFilter, RenderOptions, Renderer, Sources},
    Error, Result,
};

fn get_manifest_file(options: &RuntimeOptions) -> PathBuf {
    let mut manifest_file = options.base.clone();
    manifest_file.set_extension(config::JSON);
    manifest_file
}

#[derive(Debug)]
pub enum ProjectEntry {
    // Guaranteed to be an array with a single entry
    One(Vec<Entry>),
    // May contain multiple projects
    Many(Vec<Entry>),
}

#[derive(Debug)]
pub struct Entry {
    pub config: Config,
}

impl Entry {
    /// Get a render builder for this configuration.
    ///
    /// Creates the initial runtime options from a build profile which typically
    /// would come from command line arguments.
    ///
    /// This should only be called when you intend to render a project
    /// as it consumes the configuration entry.
    pub async fn builder(
        mut self,
        args: &ProfileSettings,
    ) -> Result<ProjectBuilder> {
        let options = crate::options::prepare(&mut self.config, args).await?;
        let redirects = if let Some(ref redirects) = self.config.redirect {
            redirects.clone()
        } else {
            Default::default()
        };

        let builder = ProjectBuilder {
            config: self.config,
            options,
            redirects,
            ..Default::default()
        };

        Ok(builder)
    }
}

/// Wrap all the collations in a vector with the guarantee that
/// it will never be empty and that the first item is the default
/// fallback locale.
#[derive(Debug, Default)]
struct CollationBuilder {
    locales: Vec<CollateInfo>,
}

impl CollationBuilder {
    fn get_fallback(&mut self) -> &mut CollateInfo {
        self.locales.iter_mut().take(1).next().unwrap()
    }

    /// Get mutable iterator over all the locales.
    ///
    /// The default fallback locale is guaranteed to be the first.
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut CollateInfo> {
        self.locales.iter_mut()
    }

    /// Get a hash map of Arc collations keyed by locale.
    fn build(mut self) -> Result<Vec<Collation>> {
        // Extract the primary fallback collation
        let fallback = self.locales.swap_remove(0);
        let fallback = Arc::new(RwLock::new(fallback));

        // Create wrappers for the other locales including
        // a pointer to the fallback collation
        let mut collations: Vec<Collation> = self
            .locales
            .into_iter()
            .map(|info| Collation {
                fallback: Arc::clone(&fallback),
                locale: Arc::new(RwLock::new(info)),
            })
            .collect();

        // Set up the default collation
        let default = Collation {
            // The primary collation just has a pointer to the fallback
            locale: Arc::clone(&fallback),
            fallback: fallback,
        };

        let mut all = vec![default];
        all.append(&mut collations);

        Ok(all)
    }
}

#[derive(Debug, Default)]
pub struct ProjectBuilder {
    locales: Locales,
    sources: Sources,
    config: Config,
    options: RuntimeOptions,
    plugins: Option<PluginCache>,
    redirects: RedirectConfig,
    datasource: DataSourceMap,
    cache: QueryCache,
    collations: CollationBuilder,
}

impl ProjectBuilder {
    /// Determine and verify input source files to compile.
    pub async fn sources(mut self) -> Result<Self> {
        debug!("Preparing sources...");

        let mut sources: Sources = Default::default();
        if let Some(ref paths) = self.options.settings.paths {
            self.verify(paths)?;
            sources.filters = Some(paths.clone());
        }
        self.sources = sources;
        Ok(self)
    }

    /// Resolve plugins.
    pub async fn plugins(mut self) -> Result<Self> {
        debug!("Resolving plugins...");

        if let Some(dependencies) = self.config.dependencies.take() {
            let plugins = plugin::resolve(
                &self.options.project,
                dependencies,
                self.options.settings.is_offline(),
            )
            .await?;

            for (_, plugin) in plugins.iter() {
                let src = plugin.source().as_ref().unwrap().to_url()?;
                info!("Use {}", plugin.to_string());
                debug!(" -> {}", src.to_string());
            }

            // Create plugin cache lookups for scripts, styles etc
            let mut plugin_cache = PluginCache::new(plugins);
            plugin_cache.prepare(self.config.engine())?;

            self.plugins = Some(plugin_cache);
        }

        Ok(self)
    }

    /// Load locale message files (.ftl).
    pub async fn locales(mut self) -> Result<Self> {
        debug!("Loading locales...");

        self.locales
            .load(&self.config, self.options.get_locales())?;
        Ok(self)
    }

    /// Verify runtime assets.
    pub async fn runtime(self) -> Result<Self> {
        debug!("Verify runtime assets...");

        if self.config.syntax.is_some() {
            if self.config.is_syntax_enabled(&self.options.settings.name) {
                let syntax_dir = dirs::syntax_dir()?;
                if !syntax_dir.exists() {
                    return Err(Error::NoSyntaxDirectory(syntax_dir));
                }
            }
        }
        Ok(self)
    }

    /// Load page front matter with inheritance, collate all files for compilation
    /// and map available links.
    pub async fn collate(mut self) -> Result<Self> {
        debug!("Collate page data...");

        let req = CollateRequest {
            locales: self.locales.languages(),
            config: &self.config,
            options: &self.options,
            plugins: self.plugins.as_ref(),
        };

        let mut res = CollateResult::new(
            &self.config.lang,
            &self.options.base,
            self.locales.languages(),
        );

        let mut errors = collator::walk(req, &mut res).await?;
        if !errors.is_empty() {
            // TODO: print all errors?
            let e = errors.swap_remove(0);
            return Err(Error::Collator(e));
        }

        let locales: Vec<CollateInfo> = res.try_into()?;
        self.collations = CollationBuilder { locales };
        Ok(self)
    }

    /// Map redirects from strings to Uris suitable for use
    /// on a local web server.
    pub async fn redirects(mut self) -> Result<Self> {
        debug!("Map redirects...");

        // Map additional redirects
        for collation in self.collations.iter_mut() {
            let redirects = collation.get_redirects();
            if !redirects.is_empty() {
                for (source, target) in redirects.iter() {
                    if self.redirects.map.contains_key(source) {
                        return Err(Error::RedirectCollision(
                            source.to_string(),
                        ));
                    }
                    self.redirects
                        .map
                        .insert(source.to_string(), target.to_string());
                }
            }
        }

        // Validate the redirects
        self.redirects.validate()?;

        Ok(self)
    }

    /// Collate plugin dependencies.
    pub async fn collate_plugins(mut self) -> Result<Self> {
        debug!("Collate plugins...");

        if let Some(ref plugin_cache) = self.plugins {
            for collation in self.collations.iter_mut() {
                plugins::collate(
                    &self.config,
                    &self.options,
                    collation,
                    plugin_cache.plugins(),
                )?;
            }
        }
        Ok(self)
    }

    /// Load data sources.
    pub async fn load_data(mut self) -> Result<Self> {
        debug!("Load collection data sources...");

        // TODO: how to iterate and store data sources?
        let collation = self.collations.get_fallback();

        // Set up the cache for data source queries
        self.cache = DataSourceMap::get_cache();

        // Load data sources and create indices
        self.datasource =
            DataSourceMap::load(&self.config, &self.options, collation).await?;

        Ok(self)
    }

    /// Create feed pages.
    pub async fn feed(mut self) -> Result<Self> {
        debug!("Collate feed pages...");

        if let Some(ref feed) = self.config.feed {
            for collation in self.collations.iter_mut() {
                collator::feed(
                    feed,
                    &self.locales,
                    &self.config,
                    &self.options,
                    self.plugins.as_ref(),
                    collation,
                )?;
            }
        }
        Ok(self)
    }

    /// Perform pagination.
    pub async fn pages(mut self) -> Result<Self> {
        debug!("Collate paginated pages...");

        for collation in self.collations.iter_mut() {
            synthetic::pages(
                &self.config,
                &self.options,
                collation,
                &self.datasource,
                &mut self.cache,
            )?;
        }
        Ok(self)
    }

    /// Create collation entries for data source iterators.
    pub async fn each(mut self) -> Result<Self> {
        debug!("Iterate collection each queries...");

        for collation in self.collations.iter_mut() {
            synthetic::each(
                &self.config,
                &self.options,
                collation,
                &self.datasource,
                &mut self.cache,
            )?;
        }
        Ok(self)
    }

    /// Create collation entries for data source assignments.
    pub async fn assign(mut self) -> Result<Self> {
        debug!("Assign query data...");

        for collation in self.collations.iter_mut() {
            synthetic::assign(
                &self.config,
                &self.options,
                collation,
                &self.datasource,
                &mut self.cache,
            )?;
        }
        Ok(self)
    }

    /// Localized pages inherit data from the fallback.
    pub async fn inherit(mut self) -> Result<Self> {
        debug!("Inherit locale page data...");

        let mut it = self.collations.locales.iter_mut();
        let fallback = it.next().unwrap();
        while let Some(collation) = it.next() {
            collation.inherit(&self.config, &self.options, fallback)?;
        }
        Ok(self)
    }

    /// Process menu references.
    pub async fn menus(mut self) -> Result<Self> {
        debug!("Compile menu references...");
        for collation in self.collations.iter_mut() {
            collation.menus =
                menu::compile(&self.config, &self.options, collation)?;
        }
        Ok(self)
    }

    /// Determine if syntax highlighting is enabled.
    pub fn get_syntax(&self) -> &Option<SyntaxConfig> {
        if self.config.is_syntax_enabled(&self.options.settings.name) {
            return &self.config.syntax;
        }
        &None
    }

    /// Setup syntax highlighting when enabled.
    pub async fn syntax(self) -> Result<Self> {
        if let Some(ref syntax_config) = self.get_syntax() {
            let syntax_dir = dirs::syntax_dir()?;
            info!("Syntax highlighting on");
            syntax::setup(&syntax_dir, syntax_config)?;
        }
        Ok(self)
    }

    pub fn build(self) -> Result<Project> {
        debug!("Creating project renderers...");

        // Set up the manifest for incremental builds
        let manifest_file = get_manifest_file(&self.options);
        let manifest = if self.options.settings.is_incremental() {
            Some(Arc::new(RwLock::new(Manifest::load(&manifest_file)?)))
        } else {
            None
        };

        let sources = Arc::new(self.sources);
        let config = Arc::new(self.config);
        let options = Arc::new(self.options);

        // Get a map of collations keyed by locale wrapper
        let collations = self.collations.build()?;

        let locales = Arc::new(self.locales);

        let plugins = if let Some(cache) = self.plugins {
            Some(Arc::new(cache))
        } else {
            None
        };

        let mut renderers: Vec<Renderer> = Vec::new();
        let mut parsers: Vec<Box<dyn Parser + Send + Sync>> = Vec::new();

        collations.into_iter().try_for_each(|collation| {
            let context = Arc::new(BuildContext {
                config: Arc::clone(&config),
                options: Arc::clone(&options),
                locales: Arc::clone(&locales),
                collation: Arc::new(RwLock::new(collation)),
                plugins: plugins.clone(),
            });

            let parser: Box<dyn Parser + Send + Sync> = parser::build(
                config.engine().clone(),
                Arc::clone(&context),
                Arc::clone(&locales),
            )?;

            // NOTE: if we need to pre-compile with the parser this is the place.

            let info = CompilerInput {
                sources: Arc::clone(&sources),
                locales: Arc::clone(&locales),
                context,
                manifest: manifest.clone(),
            };

            parsers.push(parser);
            renderers.push(Renderer::new(info));

            Ok::<(), Error>(())
        })?;

        Ok(Project {
            config,
            options,
            parsers,
            renderers,
            locales,
            manifest,
            redirects: self.redirects,
            datasource: self.datasource,
            //cache: self.cache,
        })
    }

    /// Verify the paths are within the site source.
    fn verify(&self, paths: &Vec<PathBuf>) -> Result<()> {
        for p in paths {
            if !p.starts_with(&self.options.source) {
                return Err(Error::OutsideSourceTree(p.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ProjectResult {
    sitemaps: Vec<Url>,
}

/// Project contains all the information for a render.
#[derive(Default)]
pub struct Project {
    pub config: Arc<Config>,
    pub options: Arc<RuntimeOptions>,
    pub redirects: RedirectConfig,
    pub locales: Arc<Locales>,
    pub datasource: DataSourceMap,

    //cache: QueryCache,
    parsers: Vec<Box<dyn Parser + Send + Sync>>,
    pub(crate) renderers: Vec<Renderer>,
    manifest: Option<Arc<RwLock<Manifest>>>,
}

impl Project {
    pub fn remove_file(
        &mut self,
        path: &PathBuf,
        mut lang: Option<String>,
    ) -> Result<()> {
        let lang = if let Some(lang) = lang.take() {
            lang
        } else {
            self.config.lang.clone()
        };

        // Find the correct renderer so we access the collation
        // for the language
        if let Some(renderer) = self.renderers.iter().find(|r| {
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
            } else { None };

            // Remove from the internal data structure
            collation.remove_file(path, &*self.options);

            // Now try to remove the build product
            if let Some(ref href) = href {
                let build_file = self.options.base.join(
                    utils::url::to_path_separator(
                        href.trim_start_matches("/")));

                if build_file.exists() {
                    info!("Remove {}", build_file.display());

                    if let Err(e) = fs::remove_file(&build_file) {
                        warn!(
                            "Failed to remove build file {}: {}",
                            build_file.display(), e);
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

    pub(crate) fn update_layouts(&mut self, layouts: &Vec<PathBuf>) -> Result<()> {

        //let layouts = self.context.collation.read().unwrap().layouts();
        //

        // TODO: handle new layouts
        // TODO: handle deleted layouts
        // TODO: rebuild all pages that point to a changed layout

        for layout in layouts {
            if layout.exists() {
                for parser in self.parsers.iter_mut() {
                    let name = layout.file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    parser.add(name, layout)?;
                }
            } else {
                //TODO: remove the layout from the parser!
            }
        }
        Ok(())
    }

    /// Render the project.
    pub(crate) async fn render(
        &mut self,
        render_options: RenderOptions,
    ) -> Result<ProjectResult> {
        let mut result: ProjectResult = Default::default();

        // Renderer is generated for each locale to compile
        for (parser, renderer) in self
            .parsers
            .iter()
            .zip(self.renderers.iter())
            .filter(|(_, r)| {
                let collation = r.info.context.collation.read().unwrap();
                let language = collation.get_lang();
                match render_options.filter {
                    RenderFilter::One(ref lang) => {
                        language.as_ref() == lang.as_str()
                    }
                    RenderFilter::All => true,
                }
            })
        {
            let collation = renderer.info.context.collation.read().unwrap();
            let lang = collation.get_lang().to_string();
            let collation_path = collation.get_path().to_path_buf();

            // Got a file target so we need to ensure it exists
            // in the collation otherwise it needs to be added
            if let Some(path) = render_options.file() {
                // Test file existence so we don't collide with deletion logic
                if path.exists() {
                    if collation.get_resource(path).is_none() {
                        info!("Create {} -> {}", &lang, path.display());
                        drop(collation);
                        let collation =
                            renderer.info.context.collation.write().unwrap();
                        let mut locale = collation.locale.write().unwrap();

                        let key = Arc::new(path.to_path_buf());
                        let plugins = renderer.info.context.plugins.as_deref();

                        collator::add(
                            &mut locale,
                            &*self.config,
                            &*self.options,
                            plugins,
                            &key,
                            path,
                        )?;

                        //continue;
                    } else {
                        info!("Render {} -> {}", &lang, path.display());
                    }
                }
            } else {
                info!("Render {} -> {}", &lang, collation_path.display());
            }

            let mut res = renderer.render(parser, &render_options).await?;
            if let Some(url) = res.sitemap.take() {
                result.sitemaps.push(url);
            }

            // TODO: ensure redirects work in multi-lingual config
            // TODO: respect the render_type !!!!
            self.redirects.write(&renderer.info.context.options)?;
        }

        Ok(result)
    }

    pub(crate) async fn run_hook(&self, hook: &HookConfig, changed: Option<&PathBuf>) -> Result<()> {
        for renderer in self.renderers.iter() {
            renderer.run_hook(hook, changed).await?;
        }
        Ok(())
    }

    pub fn write_manifest(&self) -> Result<()> {
        // Write the manifest for incremental builds
        if let Some(ref manifest) = self.manifest {
            let writer = manifest.write().unwrap();
            writer.save()?;
        }
        Ok(())
    }

    pub fn write_robots(&self, sitemaps: Vec<Url>) -> Result<()> {
        let output_robots =
            self.config.robots.is_some() || !sitemaps.is_empty();

        if output_robots {
            let mut robots = if let Some(ref robots) = self.config.robots {
                robots.clone()
            } else {
                Default::default()
            };

            if robots.profiles().is_match(self.options.profile())
                || !sitemaps.is_empty()
            {
                robots.sitemaps = sitemaps;

                //// NOTE: robots must always be at the root regardless
                //// NOTE: of multi-lingual support so we use `base` rather
                //// NOTE: than the `target`
                let robots_file = self.options.base.join(config::robots::FILE);
                utils::fs::write_string(&robots_file, robots.to_string())?;
                info!("Robots {}", robots_file.display());
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Workspace {
    pub projects: Vec<ProjectEntry>,
}

impl Workspace {
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }

    pub fn has_multiple_projects(&self) -> bool {
        if self.projects.len() > 1 {
            return true;
        };
        if self.projects.len() == 1 {
            return match self.projects.first().unwrap() {
                ProjectEntry::Many(_) => true,
                ProjectEntry::One(_) => false,
            };
        };
        false
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &Entry> {
        self.projects
            .iter()
            .map(|e| match e {
                ProjectEntry::One(c) | ProjectEntry::Many(c) => c.iter(),
            })
            .flatten()
            .collect::<Vec<&Entry>>()
            .into_iter()
    }

    pub fn into_iter(self) -> impl IntoIterator<Item = Entry> {
        self.projects
            .into_iter()
            .map(|e| match e {
                ProjectEntry::One(c) => c.into_iter(),
                ProjectEntry::Many(c) => c.into_iter(),
            })
            .flatten()
            .collect::<Vec<Entry>>()
            .into_iter()
    }
}

fn scm_digest(project: &PathBuf) -> Option<String> {
    if let Some(repo) = scm::discover(project).ok() {
        if let Some(rev) = repo.revparse("HEAD").ok() {
            if let Some(obj) = rev.from() {
                return Some(obj.id().to_string());
            }
        }
    }
    None
}

/// Open a project.
///
/// Load the configuration for a project and resolve workspace members when necessary.
pub fn open<P: AsRef<Path>>(dir: P, walk_ancestors: bool) -> Result<Workspace> {
    let mut workspace: Workspace = Default::default();
    let mut config = Config::load(dir.as_ref(), walk_ancestors)?;

    if let Some(ref projects) = &config.workspace {
        let mut members: Vec<Entry> = Vec::new();
        for space in &projects.members {
            let mut root = config.project().to_path_buf();
            root.push(space);
            if !root.exists() || !root.is_dir() {
                return Err(Error::NotDirectory(root));
            }

            let mut config = Config::load(&root, false)?;
            config.set_commit(scm_digest(config.project()));
            if config.workspace.is_some() {
                return Err(Error::NoNestedWorkspace(root));
            }
            members.push(Entry { config });
        }

        workspace.projects.push(ProjectEntry::Many(members));
    } else {
        config.set_commit(scm_digest(config.project()));

        workspace
            .projects
            .push(ProjectEntry::One(vec![Entry { config }]));
    }

    Ok(workspace)
}

#[derive(Default)]
pub struct CompileResult {
    pub projects: Vec<Project>,
}

/// Compile a project.
///
/// The project may contain workspace members in which case all
/// member projects will be compiled.
pub async fn compile<P: AsRef<Path>>(
    project: P,
    args: &ProfileSettings,
) -> Result<CompileResult> {
    let project = open(project, true)?;
    let mut compiled: CompileResult = Default::default();

    for entry in project.into_iter() {
        // WARN: If we add too many futures to the chain
        // WARN: then the compiler overflows resolving trait
        // WARN: bounds. The workaround is to break the chain
        // WARN: with multiple await statements.

        let builder = entry.builder(args).await?;

        // Resolve sources, locales and collate the page data
        let builder = builder
            .sources()
            .and_then(|s| s.plugins())
            .and_then(|s| s.locales())
            .and_then(|s| s.runtime())
            .and_then(|s| s.collate())
            .and_then(|s| s.inherit())
            .and_then(|s| s.collate_plugins())
            .await?;

        // Load collections, resolve synthetic assets
        let builder = builder
            .load_data()
            .and_then(|s| s.feed())
            .and_then(|s| s.menus())
            .await?;

        // Redirects come after synthetic assets in case
        // they need to create any redirects.
        let builder = builder.redirects().await?;

        // Pagination, collections, syntax highlighting
        let builder = builder
            .pages()
            .and_then(|s| s.each())
            .and_then(|s| s.assign())
            .and_then(|s| s.syntax())
            .await?;

        let mut state = builder.build()?;

        // Render all the languages
        let result = state.render(Default::default()).await?;

        // Write the robots file containing any
        // generated sitemaps
        state.write_robots(result.sitemaps)?;

        // Write out manifest for incremental builds
        state.write_manifest()?;

        compiled.projects.push(state);
    }

    Ok(compiled)
}
