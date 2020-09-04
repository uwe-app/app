use std::collections::HashMap;
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::info;

use url::Url;

use cache::CacheComponent;
use compiler::{BuildContext, CompileTarget};
use collator::manifest::Manifest;
use collator::{CollateInfo, CollateRequest, CollateResult};

use config::{Config, ProfileSettings, RuntimeOptions, RedirectConfig, LocaleName};

use datasource::{synthetic, DataSourceMap, QueryCache};

use locale::Locales;

use crate::{Error, Result, renderer::Renderer};

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

        let builder = RenderBuilder{
            context: BuildContext {
                options,
                config: self.config,
                ..Default::default()
            },
            redirects,
            ..Default::default()
        };

        Ok(builder)
    }
}

#[derive(Debug, Default)]
pub struct RenderBuilder {
    pub locales: Locales,
    pub sources: Vec<PathBuf>,
    pub targets: HashMap<LocaleName, Arc<CompileTarget>>,
    pub context: BuildContext,
    pub redirects: RedirectConfig,
    pub datasource: DataSourceMap,
    pub cache: QueryCache,
}

impl RenderBuilder {

    /// Determine and verify input source files to compile.
    pub async fn sources(mut self) -> Result<Self> {
        // Get source paths from the profile settings
        let source = self.context.options.source.clone();
        let paths: Vec<PathBuf> = if let Some(ref paths) = self.context.options.settings.paths {
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
        self.locales.load(&self.context.config, &self.context.options)?;
        let locales = self.locales.get_locale_map(&self.context.config.lang)?;

        // Set up a compile target for each locale
        let base_target = &self.context.options.base;
        for (lang, _) in locales.map.iter() {
            let target = if locales.multi {
                CompileTarget { lang: lang.clone(), path: base_target.join(lang) }
            } else {
                CompileTarget { lang: lang.clone(), path: base_target.clone() }
            };
            self.targets.insert(lang.clone(), Arc::new(target));
        } 

        self.context.options.locales = locales;

        Ok(self)
    }

    /// Fetch runtime dependencies on demand.
    pub async fn fetch(mut self) -> Result<Self> {
        let mut components: Vec<CacheComponent> = Vec::new();

        if self.context.config.syntax.is_some() {
            if self.context.config.is_syntax_enabled(&self.context.options.settings.name) {
                let syntax_dir = cache::get_syntax_dir()?;
                if !syntax_dir.exists() {
                    components.push(CacheComponent::Syntax);
                }
            }
        }

        if let Some(ref search) = self.context.config.search {
            let fetch_search_runtime = search.bundle.is_some() && search.bundle.unwrap();
            if fetch_search_runtime {
                let search_dir = cache::get_search_dir()?;
                if !search_dir.exists() {
                    components.push(CacheComponent::Search);
                }
            }
        }

        if self.context.config.feed.is_some() {
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

        // Set up the manifest for incremental builds
        let manifest_file = get_manifest_file(&self.context.options);
        let manifest: Option<Manifest> = if self.context.options.settings.is_incremental() {
            Some(Manifest::load(&manifest_file)?)
        } else {
            None
        };

        // Collate page data for later usage
        let req = CollateRequest { config: &self.context.config, options: &self.context.options };

        // FIXME: decouple the manifest from the collation

        let mut res = CollateResult::new(manifest);
        let mut errors = collator::walk(req, &mut res).await?;
        if !errors.is_empty() {
            // TODO: print all errors?
            let e = errors.swap_remove(0);
            return Err(Error::Collator(e));
        }


        let mut collation: CollateInfo = res.try_into()?;

        // Find and transform localized pages
        collator::localize(
            &self.context.config,
            &self.context.options,
            &self.context.options.locales, &mut collation).await?;

        // Collate the series data
        collator::series(&self.context.config, &self.context.options, &mut collation)?;

        self.context.collation = collation;

        Ok(self)
    }

    /// Map redirects from strings to Uris suitable for use 
    /// on a local web server.
    pub async fn redirects(mut self) -> Result<Self> {
        // Map permalink redirects
        if !self.context.collation.permalinks.is_empty() {
            for (permalink, href) in self.context.collation.permalinks.iter() {
                let key = permalink.to_string() ;
                if self.redirects.map.contains_key(&key) {
                    return Err(Error::RedirectPermalinkCollision(key));
                }
                self.redirects.map.insert(key, href.to_string());
            }
        }

        // Validate the redirects
        self.redirects.validate()?;

        Ok(self)
    }

    /// Load data sources.
    pub async fn load_data(mut self) -> Result<Self> {
        // Load data sources and create indices
        self.datasource = DataSourceMap::load(
            &self.context.config, &self.context.options, &mut self.context.collation).await?;

        // Set up the cache for data source queries
        self.cache = DataSourceMap::get_cache();

        Ok(self)
    }

    /// Copy the search runtime files if we need them.
    pub async fn search(mut self) -> Result<Self> {
        synthetic::search(&self.context.config, &self.context.options, &mut self.context.collation)?;
        Ok(self)
    }

    /// Create feed pages.
    pub async fn feed(mut self) -> Result<Self> {
        synthetic::feed(&self.context.config, &self.context.options, &mut self.context.collation)?;
        Ok(self)
    }

    /// Perform pagination.
    pub async fn pages(mut self) -> Result<Self> {
        synthetic::pages(
            &self.context.config,
            &self.context.options,
            &mut self.context.collation, &self.datasource, &mut self.cache)?;
        Ok(self)
    }

    /// Create collation entries for data source iterators.
    pub async fn each(mut self) -> Result<Self> {
        synthetic::each(
            &self.context.config,
            &self.context.options,
            &mut self.context.collation, &self.datasource, &mut self.cache)?;
        Ok(self)
    }

    /// Create collation entries for data source assignments.
    pub async fn assign(mut self) -> Result<Self> {
        synthetic::assign(
            &self.context.config,
            &self.context.options,
            &mut self.context.collation, &self.datasource, &mut self.cache)?;
        Ok(self)
    }
   
    /// Setup syntax highlighting when enabled.
    pub async fn setup_syntax(mut self) -> Result<Self> {
        if let Some(ref syntax_config) = self.context.config.syntax {
            if self.context.config.is_syntax_enabled(&self.context.options.settings.name) {
                let syntax_dir = cache::get_syntax_dir()?;
                info!("Syntax highlighting on");
                syntax::setup(&syntax_dir, syntax_config)?;
            }
        }
        Ok(self)
    }

    pub fn build(mut self) -> Result<Render> {
        let context = Arc::new(self.context);
        let sources = Arc::new(self.sources);

        let mut renderers: HashMap<LocaleName, Renderer> = HashMap::new();
        self.targets.iter()
            .try_for_each(|(lang, target)| {
                renderers.insert(
                    lang.clone(),
                    Renderer {
                        target: Arc::clone(target),
                        sources: Arc::clone(&sources),
                        context: Arc::clone(&context)
                    }
                );

                Ok::<(), Error>(())

            })?;

        Ok(Render {
            locales: self.locales,
            redirects: self.redirects,
            datasource: self.datasource,
            cache: self.cache,
            context,
            renderers,
        })

    }

    /// Verify the paths are within the site source.
    fn verify(&self, paths: &Vec<PathBuf>) -> Result<()> {
        for p in paths {
            if !p.starts_with(&self.context.options.source) {
                return Err(Error::OutsideSourceTree(p.clone()));
            }
        }
        Ok(())
    }

}

#[derive(Debug, Default)]
pub struct Render {
    pub context: Arc<BuildContext>,
    pub redirects: RedirectConfig,
    pub locales: Locales,
    pub datasource: DataSourceMap,
    pub cache: QueryCache,
    pub renderers: HashMap<LocaleName, Renderer>,
}

impl Render {

    pub fn write_redirects(&self, options: &RuntimeOptions) -> Result<()> {
        let write_redirects =
            options.settings.write_redirects.is_some()
            && options.settings.write_redirects.unwrap();

        if write_redirects {
            self.redirects.write(&options.target)?;
        }
        Ok(())
    }

    /*
    pub fn write_manifest(&mut self) -> Result<()> {
        // Write the manifest for incremental builds
        if let Some(ref mut manifest) = self.context.collation.manifest {
            let manifest_file = get_manifest_file(&self.context.options);
            for p in self.context.collation.resources.iter() {
                manifest.touch(&p.to_path_buf());
            }
            Manifest::save(&manifest_file, manifest)?;
        }
        Ok(())
    }
    */

    pub fn write_robots(&self, sitemaps: Vec<Url>) -> Result<()> {
        let output_robots = self.context.options.settings.robots.is_some()
            || !sitemaps.is_empty();

        if output_robots {
            let mut robots = if let Some(ref robots) = self.context.options.settings.robots {
                robots.clone() 
            } else {
                Default::default()
            };

            robots.sitemaps = sitemaps;

            //// NOTE: robots must always be at the root regardless
            //// NOTE: of multi-lingual support so we use `base` rather
            //// NOTE: than the `target`
            let robots_file = self.context.options.base.join(config::robots::FILE);
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
        if self.projects.len() > 1 { return true };
        if self.projects.len() == 1 {
            return match self.projects.first().unwrap() {
                ProjectEntry::Many(_) => true, 
                ProjectEntry::One(_) => false, 
            }
        };
        false
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &Entry> {
        self.projects
            .iter()
            .map(|e| {
                match e {
                    ProjectEntry::One(c) => vec![c],
                    ProjectEntry::Many(c) => c.iter().collect(),
                } 
            })
            .flatten()
            .collect::<Vec<&Entry>>()
            .into_iter()
    }

    #[deprecated(since="0.20.8", note="Use into_iter()")]
    pub fn iter_mut(&mut self) -> impl IntoIterator<Item = &mut Entry> {
        self.projects
            .iter_mut()
            .map(|e| {
                match e {
                    ProjectEntry::One(c) => vec![c],
                    ProjectEntry::Many(c) => c.iter_mut().collect(),
                } 
            })
            .flatten()
            .collect::<Vec<&mut Entry>>()
            .into_iter()
    }

    pub fn into_iter(self) -> impl IntoIterator<Item = Entry> {
        self.projects
            .into_iter()
            .map(|e| {
                match e {
                    ProjectEntry::One(c) => vec![c],
                    ProjectEntry::Many(c) => c.into_iter().collect(),
                } 
            })
            .flatten()
            .collect::<Vec<Entry>>()
            .into_iter()
    }

}

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
                return Err(Error::NoNestedWorkspace(root))
            }

            members.push(Entry { config });
        }

        workspace.projects.push(ProjectEntry::Many(members));
    } else {
        workspace.projects.push(ProjectEntry::One(Entry{ config }));
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
pub async fn compile<P: AsRef<Path>>(project: P, args: &ProfileSettings) -> Result<CompileResult> {
    let project = open(project, true)?;
    let mut compiled: CompileResult = Default::default();

    for entry in project.into_iter() {
        let mut sitemaps: Vec<Url> = Vec::new();

        let mut state = entry.builder(args)?
            .sources().await?
            .locales().await?
            .fetch().await?
            .collate().await?
            .redirects().await?
            .load_data().await?
            .search().await?
            .feed().await?
            .pages().await?
            .each().await?
            .assign().await?
            .setup_syntax().await?
            .build()?;

        // Renderer is generated for each locale to compile
        for (_lang, renderer) in state.renderers.iter() {
            let mut res = renderer.render(&state.locales).await?;
            if let Some(url) = res.sitemap.take() {
                sitemaps.push(url); 
            }
            // TODO: ensure redirects work in multi-lingual config
            state.write_redirects(&renderer.context.options)?;
        }

        // FIXME: restore manifest logic - requires decoupling from the collation
        //state.write_manifest()?;

        state.write_robots(sitemaps)?;
        compiled.projects.push(state);
    }

    Ok(compiled)
}
