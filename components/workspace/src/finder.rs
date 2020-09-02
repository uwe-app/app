use std::convert::TryInto;
use std::path::{Path, PathBuf};

use log::info;

use cache::CacheComponent;
use compiler::redirect;
use compiler::{BuildContext};
use collator::manifest::Manifest;
use collator::{CollateInfo, CollateRequest, CollateResult};

use config::{Config, ProfileSettings, RuntimeOptions};

use datasource::{synthetic, DataSourceMap, QueryCache};

use locale::Locales;

use crate::{Error, Result};
use crate::render::Render;

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
    pub fn map_options(&mut self, args: &ProfileSettings) -> Result<EntryOptions> {
        let options = crate::options::prepare(&self.config, args)?;
        Ok(EntryOptions {
            config: &mut self.config,
            options,
            locales: Default::default(),
            collation: Default::default(),
            datasource: Default::default(),
            cache: Default::default(),
        })
    }
}

#[derive(Debug)]
pub struct EntryOptions<'a> {
    pub config: &'a mut Config,
    pub options: RuntimeOptions,
    pub locales: Locales,
    pub collation: CollateInfo,
    pub datasource: DataSourceMap,
    pub cache: QueryCache,
}

impl EntryOptions<'_> {

    /// Load locale message files (.ftl).
    pub async fn load_locales(&mut self) -> Result<()> {
        self.locales.load(self.config, &self.options)?;
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
        let req = CollateRequest { config: self.config, options: &self.options };

        let mut res = CollateResult::new(manifest);
        collator::walk(req, &mut res).await?;

        let mut collation: CollateInfo = res.try_into()?;

        if !collation.errors.is_empty() {
            // TODO: print all errors?
            let e = collation.errors.swap_remove(0);
            return Err(Error::Collator(e));
        }

        // Find and transform localized pages
        collator::localize(self.config, &self.options, &self.options.locales, &mut collation).await?;

        self.collation = collation;

        Ok(())
    }

    /// Map redirects from strings to Uris suitable for use 
    /// on a local web server.
    pub async fn map_redirects(&mut self) -> Result<()> {
        // Map permalink redirects
        if !self.collation.permalinks.is_empty() {
            // Must have some redirects
            if let None = self.config.redirect {
                self.config.redirect = Some(Default::default());
            }

            if let Some(redirects) = self.config.redirect.as_mut() {
                for (permalink, href) in self.collation.permalinks.iter() {
                    let key = permalink.to_string() ;
                    if redirects.contains_key(&key) {
                        return Err(Error::RedirectPermalinkCollision(key));
                    }
                    redirects.insert(key, href.to_string());
                }
            }
        }

        // Validate the redirects
        if let Some(ref redirects) = self.config.redirect {
            redirect::validate(redirects)?;
        }

        Ok(())
    }

    /// Load data sources.
    pub async fn map_data(&mut self) -> Result<()> {
        // Load data sources and create indices
        self.datasource = DataSourceMap::load(
            self.config, &self.options, &mut self.collation).await?;

        // Set up the cache for data source queries
        self.cache = DataSourceMap::get_cache();

        Ok(())
    }

    /// Copy the search runtime files if we need them.
    pub async fn map_search(&mut self) -> Result<()> {
        Ok(synthetic::search(self.config, &self.options, &mut self.collation)?)
    }

    /// Create feed pages.
    pub async fn map_feed(&mut self) -> Result<()> {
        Ok(synthetic::feed(self.config, &self.options, &mut self.collation)?)
    }

    /// Perform pagination.
    pub async fn map_pages(&mut self) -> Result<()> {
        Ok(synthetic::pages(
            self.config, &self.options, &mut self.collation, &self.datasource, &mut self.cache)?)
    }

    /// Create collation entries for data source iterators.
    pub async fn map_each(&mut self) -> Result<()> {
        Ok(synthetic::each(
            self.config, &self.options, &mut self.collation, &self.datasource, &mut self.cache)?)
    }

    /// Create collation entries for data source assignments.
    pub async fn map_assign(&mut self) -> Result<()> {
        Ok(synthetic::assign(
            self.config, &self.options, &mut self.collation, &self.datasource, &mut self.cache)?)
    }

    pub fn to_render(self) -> Render {
        Render {
            context: BuildContext::new(self.config.clone(), self.options, self.datasource, self.collation),
            locales: self.locales,
        }
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

}

pub fn load<P: AsRef<Path>>(dir: P, walk_ancestors: bool) -> Result<Workspace> {
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
