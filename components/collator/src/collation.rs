use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use config::indexer::QueryList;
use config::{Page, RuntimeOptions};

use super::manifest::Manifest;

use super::{Error, Result};

#[derive(Debug)]
pub enum ResourceKind {
    /// A directory encountered whilst walking the tree.
    Dir,
    /// The default type indicates we don't know much about this resource.
    File,
    /// The type of file that renders to an output page.
    Page,
    /// An asset file is typically located in the `assets` folder and
    /// is primarily used for the site layout: images, fonts, styles etc.
    Asset,
    /// A locale resource, typically .ftl files in the `locales` folder.
    Locale,
    /// A partial file provides part of a template render; normally 
    /// located in the `partials` directory but can also come from 
    /// other locations.
    Partial,
    /// Include files are documents included by pages; they normally 
    /// reside in the `includes` directory and are typically used for 
    /// code samples etc.
    Include,
    /// This file is part of a data source directory.
    DataSource,
    /// Type for unknown content files such as images, videos and other 
    /// binary or text files.
    Content,
}

impl Default for ResourceKind {
    fn default() -> Self { ResourceKind::File }
}

/// The compiler uses this as the action to perform 
/// with the input source file.
#[derive(Debug)]
pub enum ResourceOperation {
    // Do nothing, used for the Dir kind primarily.
    Noop,
    // Render a file as a page template
    Render,
    // Copy a file to the build target
    Copy,
    // Create a symbolic link to the source file
    Link,
}

impl Default for ResourceOperation {
    fn default() -> Self { ResourceOperation::Copy }
}

#[derive(Debug, Default)]
pub struct ResourceTarget {
    pub destination: PathBuf,
    pub operation: ResourceOperation,
    pub kind: ResourceKind,
}

#[derive(Debug)]
pub enum Resource {
    Page { target: ResourceTarget },
    File { target: ResourceTarget },
}

impl Resource {
    pub fn new(destination: PathBuf, kind: ResourceKind, op: ResourceOperation) -> Self {
        let target = ResourceTarget {kind, destination, operation: op};
        Resource::File { target }
    }

    pub fn new_page(destination: PathBuf) -> Self {
        let kind = ResourceKind::Page;
        let target = ResourceTarget {kind, destination, operation: ResourceOperation::Render};
        Resource::Page { target }
    }

    pub fn set_operation(&mut self, operation: ResourceOperation) {
        match self {
            Self::Page {ref mut target} | Self::File {ref mut target} => {
                target.operation = operation; 
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct CollateInfo {
    /// List of errors encountered during collation.
    pub errors: Vec<Error>,

    /// All the resources resulting from a collation.
    pub all: HashMap<Arc<PathBuf>, Resource>,

    /// Lookup table for all the resources that should
    /// be processed by the compiler.
    pub resources: Vec<Arc<PathBuf>>,

    // FIXME: pages should always be keyed by locale
    /// Lookup table for page data.
    pub pages: HashMap<Arc<PathBuf>, Page>,
    /// Locale specific pages are keyed first by locale.
    pub locale_pages: HashMap<String, HashMap<Arc<PathBuf>, Page>>,

    // Pages that have permalinks map the 
    // permalink to the computed href so that
    // we can configure redirects for permalinks.
    pub permalinks: HashMap<String, String>,

    // Pages located for feed configurations.
    //
    // The hash map key is the key for the feed congfiguration 
    // and each entry is a path that can be used to 
    // locate the page data in `pages`.
    pub feeds: HashMap<String, Vec<Arc<PathBuf>>>,

    // Store queries for expansion later
    pub queries: Vec<(QueryList, Arc<PathBuf>)>,

    // List of series
    pub series: HashMap<String, Vec<Arc<PathBuf>>>,

    // Custom page specific layouts
    pub layouts: HashMap<Arc<PathBuf>, PathBuf>,
    // The default layout
    pub layout: Option<Arc<PathBuf>>,

    // TODO: books too!
    pub links: LinkMap,

    // Manifest for incremental builds
    pub manifest: Option<Manifest>,
}

impl CollateInfo {

    pub fn get_page(&self, key: &PathBuf, options: &RuntimeOptions) -> Option<&Page> {
        let mut result: Option<&Page> = None;

        if let Some(ref map) = self.locale_pages.get(&options.lang) {
            result = map.get(key);
        } 

        if result.is_none() && options.lang != options.locales.fallback {
            if let Some(ref map) = self.locale_pages.get(&options.locales.fallback) {
                result = map.get(key);
            } 
        }

        result
    }

    pub fn remove_page(&mut self, p: &PathBuf, options: &RuntimeOptions) -> Page {
        if let Some(pos) = self.resources.iter().position(|x| &**x == p) {
            self.resources.remove(pos);
        }

        // FIXME: remove using lang!
        self.pages.remove(p).unwrap()
    }

    // Rewrite destination paths.
    //
    // Used for multi-lingual output to locale specific folders.
    pub fn rewrite(&mut self, lang: &str, from: &PathBuf, to: &PathBuf) -> Result<()> {
        for (_path, page) in self.pages.iter_mut() {
            page.set_language(lang);
            page.rewrite_target(&from, &to)?;
        }

        for pth in self.resources.iter_mut() {
            let res = self.all.get_mut(pth).unwrap();
            match res {
                Resource::File {ref mut target} => {
                    let new_path = to.join(target.destination.strip_prefix(&from)?);
                    target.destination = new_path;
                }
                _ => {}
            } 
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct LinkMap {
    pub sources: HashMap<Arc<PathBuf>, Arc<String>>,
    pub reverse: HashMap<Arc<String>, Arc<PathBuf>>,
}
