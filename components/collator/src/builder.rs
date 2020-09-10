use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{loader, CollateInfo, Error, Result};
use config::{
    indexer::QueryList,
    link_utils::{self, LinkOptions},
    Config, FileInfo, FileOptions, Page, RuntimeOptions,
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
    link_utils::absolute(file, options, href_opts).map_err(Error::from)
}

/// Builds a single page and mutates the collation with necessary
/// information from the page data.
pub(crate) struct PageBuilder<'a> {
    info: &'a mut CollateInfo,
    config: &'a Config,
    options: &'a RuntimeOptions,
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
        key: &'a Arc<PathBuf>,
        path: &'a Path,
    ) -> Self {
        Self {
            info,
            config,
            options,
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

    /// Rewrite layouts relative to the source directory and register
    /// layouts with the collation.
    pub fn layouts(mut self) -> Result<Self> {
        if let Some(ref layout) = self.page.layout {
            let layout_path = self.options.source.join(layout);
            if !layout_path.exists() {
                return Err(Error::NoLayout(layout_path, layout.clone()));
            }
            self.page.layout = Some(layout_path);
        }

        if let Some(ref layout) = self.page.layout {
            self.info
                .layouts
                .insert(Arc::clone(self.key), layout.clone());
        }

        Ok(self)
    }

    /// Seal the page with file context information.
    pub fn seal(mut self) -> Result<Self> {
        let mut file_info =
            FileInfo::new(self.config, self.options, &self.path, false);

        let mut rewrite_index = self.options.settings.should_rewrite_index();
        // Override with rewrite-index page level setting
        if let Some(val) = self.page.rewrite_index {
            rewrite_index = val;
        }

        let file_opts = FileOptions {
            rewrite_index,
            base_href: &self.options.settings.base_href,
            ..Default::default()
        };

        self.rewrite_index = rewrite_index;
        self.destination = file_info.destination(&file_opts)?;
        self.page.seal(
            &self.destination,
            self.config,
            self.options,
            &file_info,
            None,
        )?;

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

            if self.info.permalinks.contains_key(&key) {
                return Err(Error::DuplicatePermalink(key));
            }

            self.info
                .permalinks
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
