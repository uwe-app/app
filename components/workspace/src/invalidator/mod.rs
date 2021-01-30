use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::PathBuf;

use config::{hook::HookConfig, FileType};

mod updater;
mod utils;

use crate::{Error, Project, Result};

use self::{
    updater::Updater,
    utils::{canonical, filter_ignores, relative_to},
};

/// Determine the kind of a change file to distinguish
/// between pages and other sorts or resources.
///
/// This is useful to determine if we should instruct
/// clients to navigate to a page when `follow-edits` is
/// enabled.
#[derive(Debug)]
pub enum Kind {
    Page(PathBuf),
    File(PathBuf),
}

#[derive(Debug)]
pub struct Invalidation {
    // Paths that are ignored but we track for debugging
    pub(crate) ignores: HashSet<PathBuf>,
    // Paths that are in the assets folders, currently we ignore these.
    pub(crate) assets: HashSet<PathBuf>,
    // Hooks are a special case so we store them separately
    pub(crate) hooks: HashSet<(HookConfig, PathBuf)>,
    // Layouts need special handling so that referenced pages
    // are also rendered
    pub(crate) layouts: HashSet<PathBuf>,
    // Partials should be re-compiled but currently we don't
    // know which files are dependent upon partials
    pub(crate) partials: HashSet<PathBuf>,
    // Includes should be collected too but we have no
    // information on which files reference the includes.
    pub(crate) includes: HashSet<PathBuf>,
    // Templates can be interspersed in the site folder but
    // must come after the tests for layout and partials and
    // behave like partials in that they are re-compiled but
    // we don't know which files reference each template
    pub(crate) templates: HashSet<PathBuf>,
    // List of actions corresponding to the files that changed
    pub(crate) actions: Vec<Kind>,
    // List of paths that do not exist anymore
    pub(crate) deletions: HashSet<PathBuf>,
    // List of paths in a collection data source
    //
    // Collections paths are stored using the identifier
    // if the corresponding `CollectionsIndex` in the
    // `CollectionsMap` so the collection can easily be located.
    pub(crate) collections: HashSet<(String, PathBuf)>,
}

impl Invalidation {
    /// Determine if this invalidation looks like a single page.
    ///
    /// Used to determine whether live reload should attempt to
    /// locate a page href (follow-edits).
    pub fn single_page(&self) -> Option<&PathBuf> {
        if self.actions.len() == 1 {
            if let Kind::Page(path) = self.actions.get(0).unwrap() {
                return Some(path);
            }
        }
        None
    }
}

pub struct Invalidator {
    updater: Updater,
}

impl Invalidator {
    pub fn new(project: Project) -> Self {
        Self {
            updater: Updater::new(project),
        }
    }

    /// Get a mutable reference to the updater.
    pub fn updater_mut(&mut self) -> &mut Updater {
        &mut self.updater
    }

    /// Try to find a page href from an invalidation path.
    ///
    /// Used by the live reload functionality to notify the browser
    /// it should navigate to the last edited page (follow-edits).
    pub fn find_page_href(&self, path: &PathBuf) -> Option<String> {
        let config = self.updater.config();
        let options = self.updater.options();
        if config.live_reload().follow_edits() {
            if let Ok(file) =
                relative_to(path, &options.source, &options.source)
            {
                for renderer in self.updater.renderers().iter() {
                    let collation =
                        renderer.info.context.collation.read().unwrap();
                    if let Some(href) = collation.get_link_href(&file) {
                        let href = href
                            .trim_end_matches(config::INDEX_HTML)
                            .to_string();
                        return Some(href);
                    }

                    drop(collation);
                }
            }
        }

        None
    }

    pub fn get_invalidation(
        &self,
        paths: HashSet<PathBuf>,
    ) -> Result<Invalidation> {
        let config = self.updater.config();
        let options = self.updater.options();

        // Collect deletions before filtering ignores
        // otherwise deleted files would be ignored and
        // removed from the paths to process.
        let (deletions, paths): (Vec<PathBuf>, Vec<PathBuf>) =
            paths.into_iter().partition(|p| !p.exists());

        let paths = filter_ignores(paths);

        let mut rule = Invalidation {
            ignores: HashSet::new(),
            assets: HashSet::new(),
            hooks: HashSet::new(),
            actions: Vec::new(),
            layouts: HashSet::new(),
            partials: HashSet::new(),
            includes: HashSet::new(),
            templates: HashSet::new(),
            deletions: deletions.into_iter().collect::<HashSet<_>>(),
            collections: HashSet::new(),
        };

        let ext = config.engine().extension().to_string();

        let config_file = config.file();
        let cfg_file = config_file.canonicalize()?;

        let hooks = if let Some(ref hooks) = config.hooks {
            hooks
                .iter()
                .filter(|h| {
                    h.has_matchers() && h.watch.is_some() && h.watch.unwrap()
                })
                .map(|h| (h, h.filter(&paths)))
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

        let build_output = canonical(options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper

        let assets = canonical(options.assets_path());
        let partials = canonical(options.partials_path());
        let includes = canonical(options.includes_path());
        let layouts = canonical(options.layouts_path());

        // FIXME: this does not respect when data sources have a `from` directory configured
        let collections = canonical(options.collections_path());

        let collections_paths: Vec<(String, PathBuf)> = self
            .updater
            .collections()
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.to_string(), canonical(v.source().clone())))
            .collect();

        'paths: for path in paths {
            match path.canonicalize() {
                Ok(path) => {
                    // NOTE: must test for hooks first as they can
                    // NOTE: point anywhere in the source directory
                    // NOTE: and should take precedence
                    for (hook, files) in hooks.iter() {
                        for f in files.iter() {
                            if &path == f {
                                rule.hooks
                                    .insert(((*hook).clone(), f.to_path_buf()));
                                continue 'paths;
                            }
                        }
                    }

                    let is_template = if let Some(extension) = path.extension()
                    {
                        extension == OsStr::new(&ext)
                    } else {
                        false
                    };

                    // This is not used at the moment but we detect it;
                    // it corresponds to the site.toml file.
                    if path == cfg_file {
                        rule.ignores.insert(path);
                    } else if path.starts_with(&layouts) {
                        rule.layouts.insert(path);
                    } else if path.starts_with(&partials) {
                        rule.partials.insert(path);
                    } else if path.starts_with(&includes) {
                        rule.includes.insert(path);
                    } else if is_template {
                        rule.templates.insert(path);

                    // Because it is valid to configure source = "."
                    // in site.toml we need to detect build output and
                    // ensure we ignore those files
                    } else if path.starts_with(&build_output) {
                        rule.ignores.insert(path);
                    } else if path.starts_with(&assets) {
                        rule.assets.insert(path);
                    } else if path.starts_with(&collections) {
                        for (key, p) in &collections_paths {
                            if path.starts_with(p) {
                                rule.collections
                                    .insert((key.to_string(), path));
                                break;
                            }
                        }
                    } else {
                        let file_type = options.get_type(&path);
                        match file_type {
                            FileType::Unknown => {
                                rule.actions.push(Kind::File(path));
                            }
                            _ => {
                                rule.actions.push(Kind::Page(path));
                            }
                        }
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }

        Ok(rule)
    }
}
