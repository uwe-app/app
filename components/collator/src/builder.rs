use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{loader, CollateInfo, Error, Result};
use config::{
    indexer::QueryList, plugin_cache::PluginCache, Config, LinkOptions, Page,
    RuntimeOptions,
};

/// Runtime validation of queries.
fn verify_query(list: &QueryList) -> Result<()> {
    let queries = list.to_vec();
    for q in queries {
        let each = q.each.is_some() && q.each.unwrap();
        if q.page.is_some() && each {
            return Err(Error::QueryConflict);
        }
    }
    Ok(())
}

/// Convert a destination path to an href path.
pub fn to_href(
    file: &PathBuf,
    options: &RuntimeOptions,
    rewrite: bool,
    strip: Option<PathBuf>,
) -> Result<String> {
    let mut href_opts: LinkOptions = Default::default();
    href_opts.strip = strip;
    href_opts.rewrite = rewrite;
    href_opts.trailing = false;
    href_opts.include_index = true;
    options.absolute(file, href_opts).map_err(Error::from)
}

/// Builds a single page and mutates the collation with necessary
/// information from the page data.
pub(crate) struct PageBuilder<'a> {
    info: &'a mut CollateInfo,
    config: &'a Config,
    options: &'a RuntimeOptions,
    plugins: Option<&'a PluginCache>,
    key: &'a Arc<PathBuf>,
    path: PathBuf,
    page: Page,
    rewrite_index: bool,
    destination: PathBuf,
}

impl<'a> PageBuilder<'a> {
    /// Create a page builder.
    ///
    /// Normally the key and path are the same however when handling locale
    /// specific overrides we need them to differ.
    pub fn new(
        info: &'a mut CollateInfo,
        config: &'a Config,
        options: &'a RuntimeOptions,
        plugins: Option<&'a PluginCache>,
        key: &'a Arc<PathBuf>,
        path: &'a Path,
    ) -> Self {
        Self {
            info,
            config,
            options,
            plugins,
            key,
            path: path.to_path_buf(),
            page: Default::default(),
            rewrite_index: false,
            destination: Default::default(),
        }
    }

    pub fn compute(mut self) -> Result<Self> {
        self.page =
            loader::compute(&self.path, self.config, self.options, true)?;
        Ok(self)
    }

    /// Extract queries from the page data and add them to the
    /// collation.
    pub fn queries(self) -> Result<Self> {
        if let Some(ref query) = self.page.query {
            // TODO: move this into the builder
            verify_query(query)?;
            self.info
                .queries
                .push((query.clone(), Arc::clone(self.key)));
        }
        Ok(self)
    }

    /// Seal the page with file context information.
    pub fn seal(mut self) -> Result<Self> {
        let mut rewrite_index = self.options.settings.should_rewrite_index();
        // Override with rewrite-index page level setting
        if let Some(val) = self.page.rewrite_index {
            rewrite_index = val;
        }

        self.rewrite_index = rewrite_index;
        self.destination = self
            .options
            .destination()
            .rewrite_index(rewrite_index)
            .build(&self.path)?;

        self.page.seal(
            self.config,
            self.options,
            &self.path,
            &self.destination,
            None,
        )?;

        Ok(self)
    }

    /// Import scripts from the scripts cache into this page.
    ///
    /// Depends on the page `href` so must come after a call to `seal()`.
    pub fn scripts(mut self) -> Result<Self> {
        if let Some(cache) = self.plugins {
            let href = self.page.href.as_ref().unwrap();
            for (dep, scripts) in cache.scripts().iter() {
                let apply = dep.apply.as_ref().unwrap();
                for matcher in apply.scripts_match.iter() {
                    if matcher.is_match(href) {
                        if self.page.scripts.is_none() {
                            self.page.scripts = Some(Vec::new());
                        }
                        if let Some(ref mut page_scripts) = self.page.scripts {
                            for s in scripts.iter().rev() {
                                page_scripts.insert(0, s.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(self)
    }

    /// Import styles from the styles cache into this page.
    ///
    /// Depends on the page `href` so must come after a call to `seal()`.
    pub fn styles(mut self) -> Result<Self> {
        if let Some(cache) = self.plugins {
            let href = self.page.href.as_ref().unwrap();
            for (dep, styles) in cache.styles().iter() {
                let apply = dep.apply.as_ref().unwrap();
                for matcher in apply.styles_match.iter() {
                    if matcher.is_match(href) {
                        if self.page.styles.is_none() {
                            self.page.styles = Some(Vec::new());
                        }
                        if let Some(ref mut page_styles) = self.page.styles {
                            for s in styles.iter().rev() {
                                page_styles.insert(0, s.clone());
                            }
                        }
                    }
                }
            }
        }
        Ok(self)
    }

    /// Import layouts from the layouts cache into this page.
    ///
    /// Depends on the page `href` so must come after a call to `seal()`.
    pub fn layouts(mut self) -> Result<Self> {
        if let Some(cache) = self.plugins {
            let href = self.page.href.as_ref().unwrap();
            for (fqn, patterns) in cache.layouts().iter() {
                for matcher in patterns.iter() {
                    if matcher.is_match(href) {
                        self.page.layout = Some(fqn.clone());
                        break;
                    }
                }
            }
        }
        Ok(self)
    }

    /// Create the link mapping for this page.
    ///
    /// Depends on `rewrite_index` so must come after a call to `seal()`.
    pub fn link(self) -> Result<Self> {
        let href = to_href(&self.path, self.options, self.rewrite_index, None)?;
        self.info
            .link(Arc::clone(self.key), Arc::new(href.clone()))?;
        Ok(self)
    }

    /// Map permalinks to be converted to redirects later.
    ///
    /// Depends on the page `href` so must come after a call to `seal()`.
    pub fn permalinks(self) -> Result<Self> {
        if let Some(ref permalink) = self.page.permalink {
            let key = permalink.trim_end_matches("/").to_string();

            if self.info.redirects.contains_key(&key) {
                return Err(Error::DuplicatePermalink(key));
            }

            self.info
                .redirects
                .insert(key, self.page.href.as_ref().unwrap().to_string());
        }
        Ok(self)
    }

    /// Collate feed pages.
    ///
    /// Depends on the page `href` so must come after a call to `seal()`.
    pub fn feeds(self) -> Result<Self> {
        if let Some(ref feed) = self.config.feed {
            for (name, cfg) in feed.channels.iter() {
                let href = self.page.href.as_ref().unwrap();
                if cfg.matcher.filter(href) {
                    let items = self
                        .info
                        .feeds
                        .entry(name.to_string())
                        .or_insert(vec![]);
                    items.push(Arc::clone(self.key));
                }
            }
        }
        Ok(self)
    }

    pub fn build(
        self,
    ) -> (&'a mut CollateInfo, &'a Arc<PathBuf>, PathBuf, Page) {
        (self.info, self.key, self.destination, self.page)
    }
}
