use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use log::info;

use futures::TryFutureExt;
use url::Url;

use cache::CacheComponent;
use collator::{
    self, menu, Collate, CollateInfo, CollateRequest, CollateResult, Collation,
};
use compiler::{parser, parser::Parser, BuildContext};

use config::{
    syntax::SyntaxConfig, Config, ProfileSettings, RedirectConfig,
    RuntimeOptions,
};

use datasource::{synthetic, DataSourceMap, QueryCache};

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
    One(Entry),
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
        let fallback = Arc::new(fallback);

        // Create wrappers for the other locales including
        // a pointer to the fallback collation
        let mut collations: Vec<Collation> = self
            .locales
            .into_iter()
            .map(|info| Collation {
                fallback: Arc::clone(&fallback),
                locale: Arc::new(info),
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
    pub locales: Locales,
    pub sources: Sources,
    pub config: Config,
    pub options: RuntimeOptions,
    pub redirects: RedirectConfig,
    pub datasource: DataSourceMap,
    pub cache: QueryCache,
    collations: CollationBuilder,
}

impl ProjectBuilder {
    /// Determine and verify input source files to compile.
    pub async fn sources(mut self) -> Result<Self> {
        let mut sources: Sources = Default::default();
        if let Some(ref paths) = self.options.settings.paths {
            self.verify(paths)?;
            sources.filters = Some(paths.clone());
        }
        self.sources = sources;
        Ok(self)
    }

    /// Load locale message files (.ftl).
    pub async fn locales(mut self) -> Result<Self> {
        self.locales.load(&self.config, &self.options)?;
        Ok(self)
    }

    /// Fetch runtime dependencies on demand.
    pub async fn fetch(self) -> Result<Self> {
        let mut components: Vec<CacheComponent> = Vec::new();

        if self.config.book.is_some() {
            let book_dir = cache::get_book_dir()?;
            if !book_dir.exists() {
                components.push(CacheComponent::Book);
            }
        }

        if self.config.syntax.is_some() {
            if self.config.is_syntax_enabled(&self.options.settings.name) {
                let syntax_dir = cache::get_syntax_dir()?;
                if !syntax_dir.exists() {
                    components.push(CacheComponent::Syntax);
                }
            }
        }

        if let Some(ref search) = self.config.search {
            let fetch_search_runtime =
                search.bundle.is_some() && search.bundle.unwrap();
            if fetch_search_runtime {
                let search_dir = cache::get_search_dir()?;
                if !search_dir.exists() {
                    components.push(CacheComponent::Search);
                }
            }
        }

        if self.config.feed.is_some() {
            let feed_dir = cache::get_feed_dir()?;
            if !feed_dir.exists() {
                components.push(CacheComponent::Feed);
            }
        }

        if !components.is_empty() {
            let prefs = preference::load()?;
            cache::update(&prefs, components)?;
        }

        Ok(self)
    }

    /// Load page front matter with inheritance, collate all files for compilation
    /// and map available links.
    pub async fn collate(mut self) -> Result<Self> {
        let req = CollateRequest {
            locales: &self.locales.languages,
            config: &self.config,
            options: &self.options,
        };

        let mut res = CollateResult::new(
            &self.config.lang,
            &self.options.base,
            &self.locales.languages,
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
        if let Some(ref plugins) = self.options.plugins {
            for collation in self.collations.iter_mut() {
                plugins::collate(&self.options, collation, plugins)?;
            }
        }
        Ok(self)
    }

    /// Load data sources.
    pub async fn load_data(mut self) -> Result<Self> {
        // TODO: how to iterate and store data sources?
        let collation = self.collations.get_fallback();

        // Set up the cache for data source queries
        self.cache = DataSourceMap::get_cache();

        // Load data sources and create indices
        self.datasource =
            DataSourceMap::load(&self.config, &self.options, collation).await?;

        Ok(self)
    }

    /// Copy the search runtime files if we need them.
    pub async fn search(mut self) -> Result<Self> {
        if let Some(ref search) = self.config.search {
            for collation in self.collations.iter_mut() {
                collator::search(
                    search,
                    &self.config,
                    &self.options,
                    collation,
                )?;
            }
        }
        Ok(self)
    }

    /// Create feed pages.
    pub async fn feed(mut self) -> Result<Self> {
        if let Some(ref feed) = self.config.feed {
            for collation in self.collations.iter_mut() {
                collator::feed(
                    feed,
                    &self.locales,
                    &self.config,
                    &self.options,
                    collation,
                )?;
            }
        }
        Ok(self)
    }

    /// Create book pages.
    pub async fn book(mut self) -> Result<Self> {
        if let Some(ref book) = self.config.book {
            for collation in self.collations.iter_mut() {
                collator::book(book, &self.config, &self.options, collation)?;
            }
        }
        Ok(self)
    }

    /// Perform pagination.
    pub async fn pages(mut self) -> Result<Self> {
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
        let mut it = self.collations.locales.iter_mut();
        let fallback = it.next().unwrap();
        while let Some(collation) = it.next() {
            collation.inherit(&self.config, &self.options, fallback)?;
        }
        Ok(self)
    }

    /// Process menu references.
    pub async fn menus(mut self) -> Result<Self> {
        for collation in self.collations.iter_mut() {
            menu::compile(&self.options, collation)?;
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
            let syntax_dir = cache::get_syntax_dir()?;
            info!("Syntax highlighting on");
            syntax::setup(&syntax_dir, syntax_config)?;
        }
        Ok(self)
    }

    pub fn build(self) -> Result<Project> {
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

        let mut renderers: Vec<Renderer> = Vec::new();
        let mut parsers: Vec<Box<dyn Parser + Send + Sync>> = Vec::new();

        collations.into_iter().try_for_each(|collation| {
            let context = Arc::new(BuildContext {
                config: Arc::clone(&config),
                options: Arc::clone(&options),
                locales: Arc::clone(&locales),
                collation: Arc::new(RwLock::new(collation)),
            });

            let parser: Box<dyn Parser + Send + Sync> = parser::build(
                config.engine(),
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
    renderers: Vec<Renderer>,
    manifest: Option<Arc<RwLock<Manifest>>>,
}

impl Project {
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
                    RenderFilter::One(ref lang) => language == lang.as_str(),
                    RenderFilter::All => true,
                }
            })
        {
            let collation = renderer.info.context.collation.read().unwrap();
            info!(
                "Render {} -> {}",
                collation.get_lang(),
                collation.get_path().display()
            );

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
            self.options.settings.robots.is_some() || !sitemaps.is_empty();

        if output_robots {
            let mut robots =
                if let Some(ref robots) = self.options.settings.robots {
                    robots.clone()
                } else {
                    Default::default()
                };

            robots.sitemaps = sitemaps;

            //// NOTE: robots must always be at the root regardless
            //// NOTE: of multi-lingual support so we use `base` rather
            //// NOTE: than the `target`
            let robots_file = self.options.base.join(config::robots::FILE);
            utils::fs::write_string(&robots_file, robots.to_string())?;
            info!("Robots {}", robots_file.display());
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
                ProjectEntry::One(c) => vec![c],
                ProjectEntry::Many(c) => c.iter().collect(),
            })
            .flatten()
            .collect::<Vec<&Entry>>()
            .into_iter()
    }

    #[deprecated(since = "0.20.8", note = "Use into_iter()")]
    pub fn iter_mut(&mut self) -> impl IntoIterator<Item = &mut Entry> {
        self.projects
            .iter_mut()
            .map(|e| match e {
                ProjectEntry::One(c) => vec![c],
                ProjectEntry::Many(c) => c.iter_mut().collect(),
            })
            .flatten()
            .collect::<Vec<&mut Entry>>()
            .into_iter()
    }

    pub fn into_iter(self) -> impl IntoIterator<Item = Entry> {
        self.projects
            .into_iter()
            .map(|e| match e {
                ProjectEntry::One(c) => vec![c],
                ProjectEntry::Many(c) => c.into_iter().collect(),
            })
            .flatten()
            .collect::<Vec<Entry>>()
            .into_iter()
    }
}

/// Open a project.
///
/// Load the configuration for a project and resolve workspace members when necessary.
pub fn open<P: AsRef<Path>>(dir: P, walk_ancestors: bool) -> Result<Workspace> {
    let mut workspace: Workspace = Default::default();
    let config = Config::load(dir.as_ref(), walk_ancestors)?;

    if let Some(ref projects) = &config.workspace {
        let mut members: Vec<Entry> = Vec::new();
        for space in &projects.members {
            let mut root = config.get_project();
            root.push(space);
            if !root.exists() || !root.is_dir() {
                return Err(Error::NotDirectory(root));
            }

            let config = Config::load(&root, false)?;
            if config.workspace.is_some() {
                return Err(Error::NoNestedWorkspace(root));
            }

            members.push(Entry { config });
        }

        workspace.projects.push(ProjectEntry::Many(members));
    } else {
        workspace.projects.push(ProjectEntry::One(Entry { config }));
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
            .and_then(|s| s.locales())
            .and_then(|s| s.fetch())
            .and_then(|s| s.collate())
            .and_then(|s| s.inherit())
            .and_then(|s| s.collate_plugins())
            .await?;

        // Load collections, resolve synthetic assets
        let builder = builder
            .load_data()
            .and_then(|s| s.search())
            .and_then(|s| s.feed())
            .and_then(|s| s.menus())
            .and_then(|s| s.book())
            .await?;

        // Redirects come after synthetic assets in case
        // they need to create any redirects. Books need
        // to redirect from the book index to the first chapter
        // for example.
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
