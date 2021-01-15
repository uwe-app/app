use std::collections::HashMap;
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use log::{debug, info, warn};

use futures::TryFutureExt;
use scopeguard::defer;
use url::Url;

use collator::{
    self, menu, CollateInfo, CollateRequest, CollateResult, Collation,
};
use compiler::{parser, parser::Parser, BuildContext};

use config::{
    hook::HookConfig, plugin_cache::PluginCache, profile::Profiles,
    server::HostConfig, syntax::SyntaxConfig, Config, ProfileSettings,
    RedirectConfig, RuntimeOptions,
};

use collections::{synthetic, DataSourceMap, QueryCache};

use locale::Locales;

use crate::{
    lock,
    manifest::Manifest,
    plugins,
    renderer::{CompilerInput, RenderFilter, RenderOptions, Renderer, Sources},
    Error, Result,
};

static PLUGIN_SYNTAX: &str = "std::syntax";

fn get_manifest_file(options: &RuntimeOptions) -> PathBuf {
    let mut manifest_file = options.base.clone();
    manifest_file.set_extension(config::JSON);
    manifest_file
}

/// Workspace member cache with key and host name.
#[derive(Debug)]
pub struct Member {
    pub key: String,
    pub hostname: String,
}

impl Member {
    pub fn new(key: String, hostname: String) -> Self {
        Self { key, hostname }
    }
}

#[derive(Debug)]
pub enum Workspace {
    /// Represents a single project.
    ///
    /// Even though this is only a single item we wrap it in a
    /// vector to make iteration logic simpler.
    One(Vec<Config>),
    /// Represents a workspace with multiple projects.
    ///
    /// The second entry in the tuple is the settings for the workspace
    /// and the last entry if a list of members names to be filtered.
    Many(Vec<Config>, Config, Vec<String>),
}

impl Workspace {
    pub fn is_empty(&self) -> bool {
        match self {
            Workspace::One(c) | Workspace::Many(c, _, _) => c.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Workspace::One(c) | Workspace::Many(c, _, _) => c.len(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Config> {
        match self {
            Workspace::One(c) | Workspace::Many(c, _, _) => c.iter(),
        }
    }

    pub fn into_iter(self) -> impl IntoIterator<Item = Config> {
        match self {
            Workspace::One(c) | Workspace::Many(c, _, _) => c.into_iter(),
        }
    }

    pub fn member_filters(&self) -> Vec<String> {
        match self {
            Workspace::One(_) => vec![],
            Workspace::Many(_, _, ref member_filters) => member_filters.clone(),
        }
    }
}

/// Get a project builder for a configuration.
///
/// Creates the initial runtime options from a build profile which typically
/// would come from command line arguments.
///
/// This should only be called when you intend to render a project
/// as it consumes the configuration entry.
pub async fn new_project_builder(
    mut config: Config,
    args: &ProfileSettings,
    members: &Vec<Member>,
) -> Result<ProjectBuilder> {
    let options = crate::options::prepare(&mut config, args, members).await?;
    let redirects = if let Some(ref redirects) = config.redirect {
        redirects.clone()
    } else {
        Default::default()
    };

    let builder = ProjectBuilder {
        config: config,
        options,
        redirects,
        ..Default::default()
    };

    Ok(builder)
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

        if let Some(ref dependencies) = self.config.dependencies() {
            if !dependencies.is_empty() {
                let mut plugins = plugin::install(&self.config).await?;

                // Prepare the dependencies
                for (dep, plugin) in plugins.iter_mut() {
                    info!("Use {}", plugin);
                    debug!(" -> {}", plugin.base().display());
                    let src = plugin.source().as_ref().unwrap().to_url()?;
                    debug!(" -> {}", src.to_string());

                    // Prepare the dependency so that we have cached
                    // glob matchers and so that we can expand apply
                    // shorthand notation (TODO)
                    dep.prepare()?;
                }

                // Create plugin cache lookups for scripts, styles etc
                let mut plugin_cache = PluginCache::new(plugins);
                plugin_cache.prepare(self.config.engine())?;

                self.plugins = Some(plugin_cache);
            }
        }
        Ok(self)
    }

    /// Load locale message files (.ftl).
    pub async fn locales(mut self) -> Result<Self> {
        debug!("Loading locales...");

        self.locales
            .load(&self.config, self.options.locales_path())?;
        Ok(self)
    }

    /// Verify runtime assets.
    pub async fn runtime(self) -> Result<Self> {
        debug!("Verify runtime assets...");

        if self.config.syntax.is_some() {
            if self.config.is_syntax_enabled(&self.options.settings.name) {
                if let Some(ref plugin_cache) = self.plugins {
                    if let Some(plugin) = plugin_cache.find(PLUGIN_SYNTAX) {
                        let syntax_dir = plugin.base();
                        if !syntax_dir.exists() || !syntax_dir.is_dir() {
                            return Err(Error::NoSyntaxDirectory(
                                syntax_dir.to_path_buf(),
                            ));
                        }
                    } else {
                        return Err(Error::NoSyntaxPlugin(
                            PLUGIN_SYNTAX.to_string(),
                        ));
                    }
                } else {
                    return Err(Error::NoSyntaxPlugin(
                        PLUGIN_SYNTAX.to_string(),
                    ));
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
            let plugin_cache = self.plugins.as_ref().unwrap();
            let syntax_plugin = plugin_cache.find(PLUGIN_SYNTAX).unwrap();
            info!("Syntax highlighting on");
            syntax::setup(syntax_plugin.base(), syntax_config)?;
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
    pub fn config(&self) -> &Config {
        &*self.config
    }

    pub fn options(&self) -> &RuntimeOptions {
        &*self.options
    }

    pub fn collections(&self) -> &DataSourceMap {
        &self.datasource
    }

    pub fn parsers_mut(&mut self) -> &mut Vec<Box<dyn Parser + Send + Sync>> {
        &mut self.parsers
    }

    pub fn renderers(&self) -> &Vec<Renderer> {
        &self.renderers
    }

    pub fn renderers_mut(&mut self) -> &mut Vec<Renderer> {
        &mut self.renderers
    }

    pub fn iter_mut(
        &mut self,
    ) -> std::iter::Zip<
        std::slice::IterMut<
            '_,
            Box<(dyn Parser + Sync + std::marker::Send + 'static)>,
        >,
        std::slice::IterMut<'_, Renderer>,
    > {
        self.parsers.iter_mut().zip(self.renderers.iter_mut())
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

    pub(crate) async fn run_hook(
        &self,
        hook: &HookConfig,
        changed: Option<&PathBuf>,
    ) -> Result<()> {
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

fn scm_digest(project: &PathBuf) -> Option<String> {
    if let Some(repo) = scm::discover(project).ok() {
        return scm::last_commit(&repo, scm::HEAD).map(|oid| oid.to_string());
    }
    None
}

/// Open a project.
///
/// Load the configuration for a project and resolve workspace members when necessary.
pub fn open<P: AsRef<Path>>(
    dir: P,
    walk_ancestors: bool,
    member_filters: &Vec<String>,
) -> Result<Workspace> {
    let mut config = Config::load(dir.as_ref(), walk_ancestors)?;

    if let Some(ref projects) = &config.workspace {
        let mut members: Vec<Config> = Vec::new();
        let mut names: HashMap<String, PathBuf> = HashMap::new();

        let multiple = projects.members.len() > 1;

        for space in projects.members.iter() {
            let mut root = config.project().to_path_buf();
            root.push(space);
            if !root.exists() || !root.is_dir() {
                return Err(Error::NotDirectory(root));
            }

            let mut config = Config::load(&root, false)?;

            let hostname = config.get_local_host_name(multiple);
            if let Some(ref existing_project) = names.get(&hostname) {
                return Err(Error::DuplicateHostName(
                    hostname.clone(),
                    existing_project.to_path_buf(),
                    config.project().clone(),
                ));
            }
            names.insert(hostname.clone(), config.project().clone());

            config.set_commit(scm_digest(config.project()));
            config.set_member_name(space);
            if config.workspace.is_some() {
                return Err(Error::NoNestedWorkspace(root));
            }
            members.push(config);
        }

        Ok(Workspace::Many(members, config, member_filters.clone()))
    } else {
        config.set_commit(scm_digest(config.project()));
        Ok(Workspace::One(vec![config]))
    }
}

/// Get the settings for a project.
///
/// For workspaces a list of member settings is also returned.
pub fn settings<P: AsRef<Path>>(
    dir: P,
    walk_ancestors: bool,
    member_filters: &Vec<String>,
) -> Result<(Config, Option<Vec<Config>>)> {
    let workspace = open(dir, walk_ancestors, member_filters)?;
    match workspace {
        Workspace::One(mut entries) => {
            Ok((entries.swap_remove(0).into(), None))
        }
        Workspace::Many(entries, config, _) => Ok((config, Some(entries))),
    }
}

/// Wrapper for project that can be used to create
/// a host configuration.
pub struct HostInfo {
    pub name: String,
    pub project: Project,
    pub source: PathBuf,
    pub target: PathBuf,
    pub endpoint: String,
}

#[derive(Default)]
pub struct CompileResult {
    pub projects: Vec<Project>,
}

impl Into<HostResult> for CompileResult {
    fn into(self) -> HostResult {
        // Multiple projects will use *.localhost names
        // otherwise we can just run using the standard `localhost`.
        let multiple = self.projects.len() > 1;
        let hosts: Vec<HostInfo> = self
            .projects
            .into_iter()
            .map(|project| {
                let name = project.config.get_local_host_name(multiple);
                let source = project.options.source.clone();
                let target = project.options.base.clone();
                let endpoint = utils::generate_id(16);
                HostInfo {
                    name,
                    project,
                    source,
                    target,
                    endpoint,
                }
            })
            .collect();

        HostResult { hosts }
    }
}

#[derive(Default)]
pub struct HostResult {
    pub hosts: Vec<HostInfo>,
}

impl TryInto<Vec<(HostInfo, HostConfig)>> for HostResult {
    type Error = crate::Error;
    fn try_into(
        self,
    ) -> std::result::Result<Vec<(HostInfo, HostConfig)>, Self::Error> {
        let mut out: Vec<(HostInfo, HostConfig)> = Vec::new();

        self.hosts.into_iter().try_for_each(|info| {
            let target = info.target.clone();
            let hostname = info.name.clone();
            let endpoint = info.endpoint.clone();
            let redirect_uris = info.project.redirects.collect()?;

            info!(
                "Virtual host: {} ({} redirects)",
                &hostname,
                redirect_uris.len()
            );

            let host = HostConfig::new(
                target,
                hostname,
                Some(redirect_uris),
                Some(endpoint),
                false,
                false,
            );

            out.push((info, host));

            Ok::<(), Error>(())
        })?;
        Ok(out)
    }
}

/// Compile a project.
///
/// The project may contain workspace members in which case all
/// member projects will be compiled.
pub async fn compile<P: AsRef<Path>>(
    project: P,
    args: &ProfileSettings,
) -> Result<CompileResult> {
    let workspace = open(project.as_ref(), true, &args.member)?;

    let mut compiled: CompileResult = Default::default();

    // Cache of workspace member information used to
    // build URLs for linking to members in templates
    let members: Vec<Member> = match &workspace {
        Workspace::Many(configs, _, _) => configs
            .iter()
            .map(|c| {
                Member::new(
                    c.member_name().as_ref().unwrap().to_owned(),
                    c.host().to_owned(),
                )
            })
            .collect(),
        _ => vec![],
    };

    let member_filters = workspace.member_filters();

    for config in workspace.into_iter() {
        if let Some(member_name) = config.member_name() {
            if !member_filters.is_empty()
                && !member_filters.contains(member_name)
            {
                continue;
            }
        }

        let lock_path = config.file().to_path_buf();
        let lock_file = lock::acquire(&lock_path)?;
        defer! { let _ = lock::release(lock_file); }

        // WARN: If we add too many futures to the chain
        // WARN: then the compiler overflows resolving trait
        // WARN: bounds. The workaround is to break the chain
        // WARN: with multiple await statements.

        if config.hooks.is_some() && !args.can_exec() {
            warn!("The project has some hooks defined but does ");
            warn!("not have the capability to execute commands.");
            warn!("");
            warn!("{}", config.file().display());
            warn!("");
            warn!("If you trust the commands in the site settings ");
            warn!("enable command execution with the --exec option.");
            warn!("");
            return Err(Error::NoExecCapability(config.host.to_string()));
        }

        // Prepare the options and project builder
        let builder = new_project_builder(config, args, &members).await?;

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
        let builder = builder.load_data().and_then(|s| s.menus()).await?;

        // Redirects come after synthetic assets in case
        // they need to create any redirects.
        let builder = builder.redirects().await?;

        // Pagination, collections, syntax highlighting
        let builder = builder
            .pages()
            .and_then(|s| s.each())
            .and_then(|s| s.assign())
            .and_then(|s| s.syntax())
            // NOTE: feed comes after synthetic collections
            // NOTE: so that <link rel="alternate"> patterns
            // NOTE: can be injected correctly
            .and_then(|s| s.feed())
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
