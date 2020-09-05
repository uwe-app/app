use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::info;

use futures::TryFutureExt;
use url::Url;

use cache::CacheComponent;
//use collator::manifest::Manifest;
use collator::{CollateInfo, Collation};
use compiler::{BuildContext, CompileInfo};

use config::{
    Config, LocaleName, ProfileSettings, RedirectConfig, RuntimeOptions,
};

use datasource::{synthetic, DataSourceMap, QueryCache};

use locale::Locales;

use crate::{collation, renderer::Renderer, Error, Result};

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
    pub fn builder(self, args: &ProfileSettings) -> Result<RenderBuilder> {
        let options = crate::options::prepare(&self.config, args)?;
        let redirects = if let Some(ref redirects) = self.config.redirect {
            redirects.clone()
        } else {
            Default::default()
        };

        let builder = RenderBuilder {
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

        //map.insert(default.locale.lang.clone(), default);
        //collations.into_iter()
        //.for_each(|info| {
        //map.insert(info.locale.lang.clone(), info);
        //});

        Ok(all)
    }
}

#[derive(Debug, Default)]
pub struct RenderBuilder {
    pub locales: Locales,
    pub sources: Vec<PathBuf>,
    pub config: Config,
    pub options: RuntimeOptions,
    pub redirects: RedirectConfig,
    pub datasource: DataSourceMap,
    pub cache: QueryCache,
    collations: CollationBuilder,
}

impl RenderBuilder {
    /// Determine and verify input source files to compile.
    pub async fn sources(mut self) -> Result<Self> {
        // Get source paths from the profile settings
        let source = self.options.source.clone();
        let paths: Vec<PathBuf> =
            if let Some(ref paths) = self.options.settings.paths {
                self.verify(paths)?;
                paths.clone()
            } else {
                vec![source]
            };

        self.sources = paths;

        Ok(self)
    }

    /// Load locale message files (.ftl).
    pub async fn locales(mut self) -> Result<Self> {
        self.locales.load(&self.config, &self.options)?;
        let locales = self.locales.get_locale_map(&self.config.lang)?;
        self.options.locales = locales;
        Ok(self)
    }

    /// Fetch runtime dependencies on demand.
    pub async fn fetch(self) -> Result<Self> {
        let mut components: Vec<CacheComponent> = Vec::new();

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
        // FIXME: restore manifest handling?
        // Set up the manifest for incremental builds
        /*
        let manifest_file = get_manifest_file(&self.options);
        let manifest: Option<Manifest> = if self.options.settings.is_incremental() {
            Some(Manifest::load(&manifest_file)?)
        } else {
            None
        };
        */

        // Get a reference to the locale map
        let locales = &self.options.locales;
        let config = &self.config;
        let options = &self.options;

        let mut fallback = collation::collate(locales, config, options).await?;

        let languages = locales
            .map
            .keys()
            .filter(|lang| lang != &&locales.fallback)
            .map(|s| s.as_str())
            .collect::<Vec<_>>();

        let mut values = collation::extract(
            locales,
            &mut fallback,
            languages,
            config,
            options,
        )
        .await?;

        let mut locales = vec![fallback];
        locales.append(&mut values);
        self.collations = CollationBuilder { locales };

        Ok(self)
    }

    /// Map redirects from strings to Uris suitable for use
    /// on a local web server.
    pub async fn redirects(mut self) -> Result<Self> {
        // Map permalink redirects
        for collation in self.collations.iter_mut() {
            if !collation.permalinks.is_empty() {
                for (permalink, href) in collation.permalinks.iter() {
                    let key = permalink.to_string();
                    if self.redirects.map.contains_key(&key) {
                        return Err(Error::RedirectPermalinkCollision(key));
                    }
                    self.redirects.map.insert(key, href.to_string());
                }
            }
        }

        // Validate the redirects
        self.redirects.validate()?;

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
        for collation in self.collations.iter_mut() {
            synthetic::search(&self.config, &self.options, collation)?;
        }

        Ok(self)
    }

    /// Create feed pages.
    pub async fn feed(mut self) -> Result<Self> {
        for collation in self.collations.iter_mut() {
            synthetic::feed(&self.config, &self.options, collation)?;
        }
        Ok(self)
    }

    /// Collate series data.
    pub async fn series(mut self) -> Result<Self> {
        for collation in self.collations.iter_mut() {
            collator::series(&self.config, &self.options, collation)?;
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

    pub fn has_syntax(&self) -> bool {
        self.config.syntax.is_some()
            && self.config.is_syntax_enabled(&self.options.settings.name)
    }

    /// Setup syntax highlighting when enabled.
    pub async fn syntax(self) -> Result<Self> {
        if let Some(ref syntax_config) = self.config.syntax {
            if self.config.is_syntax_enabled(&self.options.settings.name) {
                let syntax_dir = cache::get_syntax_dir()?;
                info!("Syntax highlighting on");
                syntax::setup(&syntax_dir, syntax_config)?;
            }
        }
        Ok(self)
    }

    pub fn build(self) -> Result<Render> {
        let sources = Arc::new(self.sources);
        let config = Arc::new(self.config);
        let options = Arc::new(self.options);

        // Get a map of collations keyed by locale wrapper
        let collations = self.collations.build()?;

        let mut renderers: HashMap<LocaleName, Renderer> = HashMap::new();
        collations.into_iter().try_for_each(|collation| {
            let lang = collation.locale.lang.clone();
            let context = BuildContext {
                config: Arc::clone(&config),
                options: Arc::clone(&options),
                collation: Arc::new(collation),
            };

            let info = CompileInfo {
                sources: Arc::clone(&sources),
                context,
            };
            renderers.insert(lang, Renderer { info });
            Ok::<(), Error>(())
        })?;

        Ok(Render {
            config,
            options,
            renderers,
            locales: self.locales,
            redirects: self.redirects,
            datasource: self.datasource,
            cache: self.cache,
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
pub struct Render {
    pub config: Arc<Config>,
    pub options: Arc<RuntimeOptions>,
    pub redirects: RedirectConfig,
    pub locales: Locales,
    pub datasource: DataSourceMap,
    pub cache: QueryCache,
    pub renderers: HashMap<LocaleName, Renderer>,
}

impl Render {
    pub fn get_fallback_context(&self) -> &BuildContext {
        &self.get_fallback_renderer().info.context
    }

    pub fn get_fallback_renderer(&self) -> &Renderer {
        self.renderers.get(&self.config.lang).unwrap()
    }

    pub fn write_redirects(&self, options: &RuntimeOptions) -> Result<()> {
        let write_redirects = options.settings.write_redirects.is_some()
            && options.settings.write_redirects.unwrap();

        if write_redirects {
            self.redirects.write(&options.target)?;
        }
        Ok(())
    }

    /*
    pub fn write_manifest(&mut self) -> Result<()> {
        // Write the manifest for incremental builds
        if let Some(ref mut manifest) = self.collation.manifest {
            let manifest_file = get_manifest_file(&self.options);
            for p in self.collation.resources.iter() {
                manifest.touch(&p.to_path_buf());
            }
            Manifest::save(&manifest_file, manifest)?;
        }
        Ok(())
    }
    */

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

#[derive(Debug, Default)]
pub struct CompileResult {
    pub projects: Vec<Render>,
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
        let mut sitemaps: Vec<Url> = Vec::new();

        let builder = entry.builder(args)?;
        let builder = builder
            .sources()
            .and_then(|s| s.locales())
            .and_then(|s| s.fetch())
            .and_then(|s| s.collate())
            .and_then(|s| s.redirects())
            .and_then(|s| s.load_data())
            .and_then(|s| s.search())
            .and_then(|s| s.feed())
            .and_then(|s| s.series())
            .and_then(|s| s.pages())
            .and_then(|s| s.each())
            .and_then(|s| s.assign())
            .await?;

        // WARN: If we add the future from syntax() to the chain
        // WARN: above then the compiler overflows resolving trait
        // WARN: bounds. The workaround is to await (above) and
        // WARN: then await again here.
        let builder = if builder.has_syntax() {
            builder.syntax().await?
        } else {
            builder
        };

        let state = builder.build()?;

        // Renderer is generated for each locale to compile
        for (_lang, renderer) in state.renderers.iter() {
            let mut res = renderer.render(&state.locales).await?;
            if let Some(url) = res.sitemap.take() {
                sitemaps.push(url);
            }
            // TODO: ensure redirects work in multi-lingual config
            state.write_redirects(&renderer.info.context.options)?;
        }

        // FIXME: restore manifest logic - requires decoupling from the collation
        //state.write_manifest()?;

        state.write_robots(sitemaps)?;
        compiled.projects.push(state);
    }

    Ok(compiled)
}
