use std::collections::HashMap;
use std::convert::TryInto;
use std::path::{Path, PathBuf};

use log::info;

use url::Url;

use cache::CacheComponent;
use compiler::BuildContext;
use collator::manifest::Manifest;
use collator::{CollateInfo, CollateRequest, CollateResult};

use config::{Config, ProfileSettings, RuntimeOptions, RedirectConfig};

use datasource::{synthetic, DataSourceMap, QueryCache};

use locale::{Locales, LocaleName};

use crate::{Error, Result, render::Renderer};

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

    /// Get a render state for this configuration.
    ///
    /// Creates the initial runtime options from a build profile which typically 
    /// would come from command line arguments.
    ///
    /// This should only be called when you intend to render a project 
    /// as it consumes the configuration entry.
    pub fn from_profile(self, args: &ProfileSettings) -> Result<RenderState> {
        let options = crate::options::prepare(&self.config, args)?;
        let redirects = if let Some(ref redirects) = self.config.redirect {
            redirects.clone()    
        } else {
            Default::default()
        };

        Ok(RenderState {
            config: self.config,
            options,
            collation: Default::default(),
            redirects,
            locales: Default::default(),
            datasource: Default::default(),
            cache: Default::default(),
            renderers: Default::default(),
        })
    }
}

#[derive(Debug, Default)]
pub struct RenderState {
    pub config: Config,
    pub options: RuntimeOptions,
    pub collation: CollateInfo,
    pub redirects: RedirectConfig,
    pub locales: Locales,
    pub datasource: DataSourceMap,
    pub cache: QueryCache,
    pub renderers: HashMap<LocaleName, Renderer>,
}

impl RenderState {

    /// Load locale message files (.ftl).
    pub async fn load_locales(&mut self) -> Result<()> {
        self.locales.load(&self.config, &self.options)?;
        let locale_map = self.locales.get_locale_map(&self.config.lang)?;
        self.options.locales = locale_map;
        Ok(())
    }

    /// Fetch runtime dependencies on demand.
    pub async fn fetch_lazy(&mut self) -> Result<()> {

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
            let fetch_search_runtime = search.bundle.is_some() && search.bundle.unwrap();
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

        Ok(())
    }

    /// Setup syntax highlighting when enabled.
    pub async fn map_syntax(&mut self) -> Result<()> {
        if let Some(ref syntax_config) = self.config.syntax {
            if self.config.is_syntax_enabled(&self.options.settings.name) {
                let syntax_dir = cache::get_syntax_dir()?;
                info!("Syntax highlighting on");
                syntax::setup(&syntax_dir, syntax_config)?;
            }
        }
        Ok(())
    }

    /// Load page front matter with inheritance, collate all files for compilation 
    /// and map available links.
    pub async fn collate(&mut self) -> Result<()> {

        // Set up the manifest for incremental builds
        let manifest_file = get_manifest_file(&self.options);
        let manifest: Option<Manifest> = if self.options.settings.is_incremental() {
            Some(Manifest::load(&manifest_file)?)
        } else {
            None
        };

        // Collate page data for later usage
        let req = CollateRequest { config: &self.config, options: &self.options };

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
        collator::localize(&self.config, &self.options, &self.options.locales, &mut collation).await?;

        // Collate the series data
        collator::series(&self.config, &self.options, &mut collation)?;

        self.collation = collation;

        Ok(())
    }

    /// Map redirects from strings to Uris suitable for use 
    /// on a local web server.
    pub async fn map_redirects(&mut self) -> Result<()> {
        // Map permalink redirects
        if !self.collation.permalinks.is_empty() {
            // Must have some redirects
            //if let None = self.config.redirect {
                //self.config.redirect = Some(Default::default());
            //}

            //if let Some(redirects) = self.config.redirect.as_mut() {
                for (permalink, href) in self.collation.permalinks.iter() {
                    let key = permalink.to_string() ;
                    if self.redirects.map.contains_key(&key) {
                        return Err(Error::RedirectPermalinkCollision(key));
                    }
                    self.redirects.map.insert(key, href.to_string());
                }
            //}
        }

        // Validate the redirects
        //if let Some(ref redirects) = self.config.redirect {
        //redirect::validate(&self.redirects.map)?;
        //}

        self.redirects.validate()?;

        Ok(())
    }

    /// Load data sources.
    pub async fn map_data(&mut self) -> Result<()> {
        // Load data sources and create indices
        self.datasource = DataSourceMap::load(
            &self.config, &self.options, &mut self.collation).await?;

        // Set up the cache for data source queries
        self.cache = DataSourceMap::get_cache();

        Ok(())
    }

    /// Copy the search runtime files if we need them.
    pub async fn map_search(&mut self) -> Result<()> {
        Ok(synthetic::search(&self.config, &self.options, &mut self.collation)?)
    }

    /// Create feed pages.
    pub async fn map_feed(&mut self) -> Result<()> {
        Ok(synthetic::feed(&self.config, &self.options, &mut self.collation)?)
    }

    /// Perform pagination.
    pub async fn map_pages(&mut self) -> Result<()> {
        Ok(synthetic::pages(
            &self.config, &self.options, &mut self.collation, &self.datasource, &mut self.cache)?)
    }

    /// Create collation entries for data source iterators.
    pub async fn map_each(&mut self) -> Result<()> {
        Ok(synthetic::each(
            &self.config, &self.options, &mut self.collation, &self.datasource, &mut self.cache)?)
    }

    /// Create collation entries for data source assignments.
    pub async fn map_assign(&mut self) -> Result<()> {
        Ok(synthetic::assign(
            &self.config, &self.options, &mut self.collation, &self.datasource, &mut self.cache)?)
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

    /// Get the build context for a compiler pass.
    pub fn to_context(&self) -> BuildContext {
        // FIXME: must remove the clones here and 
        // FIXME: pass an Arc to the compiler
        BuildContext::new(
            self.config.clone(),
            self.options.clone(),
            self.collation.clone(),
        )
    }

    /// Get a list of renderers for each locale. 
    pub fn renderer(&mut self) -> Result<()> {
        let locales = self.options.locales.clone();
        let mut options = self.options.clone();
        let base_target = options.target.clone();

        locales.map.keys()
            .try_for_each(|lang| {
                let mut context = self.to_context();

                if locales.multi {
                    let locale_target = base_target.join(lang);
                    options.lang = lang.clone();
                    options.target = locale_target.clone();
                    context.collation.rewrite(&options, lang, &base_target, &locale_target)?;
                }

                let paths: Vec<PathBuf> = if let Some(ref paths) = context.options.settings.paths {
                    self.verify(paths)?;
                    paths.clone()
                } else {
                    vec![options.source.clone()]
                };

                self.renderers.insert(lang.clone(), Renderer {context, paths});

                Ok::<(), Error>(())
            })?;

        Ok(())
    }

    pub fn write_redirects(&self, options: &RuntimeOptions) -> Result<()> {
        let write_redirects =
            options.settings.write_redirects.is_some()
            && options.settings.write_redirects.unwrap();

        if write_redirects {
            self.redirects.write(&options.target)?;
        }
        Ok(())
    }

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

    pub fn write_robots(&self, sitemaps: Vec<Url>) -> Result<()> {
        let output_robots = self.options.settings.robots.is_some()
            || !sitemaps.is_empty();

        if output_robots {
            let mut robots = if let Some(ref robots) = self.options.settings.robots {
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
    pub projects: Vec<RenderState>,
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
        let mut state = entry.from_profile(args)?;

        state.load_locales().await?;
        state.fetch_lazy().await?;

        state.collate().await?;

        state.map_redirects().await?;
        state.map_data().await?;

        state.map_search().await?;
        state.map_feed().await?;

        state.map_pages().await?;
        state.map_each().await?;
        state.map_assign().await?;

        // TODO: do this after fetch_lazy() ?
        state.map_syntax().await?;

        let mut sitemaps: Vec<Url> = Vec::new();

        state.renderer()?;

        // Renderer is generated for each locale to compile
        for (_lang, renderer) in state.renderers.iter() {
            let mut res = renderer.render(&state.locales).await?;
            if let Some(url) = res.sitemap.take() {
                sitemaps.push(url); 
            }

            // TODO: ensure redirects work in multi-lingual config
            state.write_redirects(&renderer.context.options)?;

        }

        state.write_manifest()?;

        state.write_robots(sitemaps)?;
        compiled.projects.push(state);
    }

    Ok(compiled)
}
