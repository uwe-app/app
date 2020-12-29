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

/*
 *  Invalidation rules.
 *
 *  - Page: rebuild the page.
 *  - File: copy the file to build.
 *  - CollectionsDocument: TODO.
 */
#[derive(Debug)]
pub enum Action {
    Page(PathBuf),
    File(PathBuf),
    CollectionsDocument(PathBuf),
}

#[derive(Debug)]
pub struct Rule {
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
    // Templates can be interspersed in the site folder but
    // must come after the tests for layout and partials and
    // behave like partials in that they are re-compiled but
    // we don't know which files reference each template
    pub(crate) templates: HashSet<PathBuf>,
    // List of actions corresponding to the files that changed
    pub(crate) actions: Vec<Action>,
    // List of paths that do not exist anymore
    pub(crate) deletions: HashSet<PathBuf>,
}

impl Rule {
    /// Determine if this invalidation looks like a single page.
    ///
    /// Used to determine whether live reload should attempt to
    /// locate a page href (follow-edits).
    pub fn single_page(&self) -> Option<&PathBuf> {
        if self.actions.len() == 1 {
            if let Action::Page(path) = self.actions.get(0).unwrap() {
                return Some(path);
            }
        }
        None
    }
}

pub struct Invalidator<'a> {
    updater: Updater<'a>,
}

impl<'a> Invalidator<'a> {
    pub fn new(project: &'a mut Project) -> Self {
        Self {
            updater: Updater::new(project),
        }
    }

    /// Try to find a page href from an invalidation path.
    ///
    /// Used by the live reload functionality to notify the browser
    /// it should navigate to the last edited page (follow-edits).
    pub fn find_page_href(&mut self, path: &PathBuf) -> Option<String> {
        let project = self.updater.project();
        let source = project.options.source.clone();
        if project.config.livereload().follow_edits() {
            if let Ok(file) = relative_to(path, &source, &source) {
                for renderer in project.renderers.iter() {
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

    pub fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Rule> {
        let project = self.updater.project();

        // Collect deletions before filtering ignores
        // otherwise deleted files would be ignored and
        // removed from the paths to process.
        let (deletions, paths): (Vec<PathBuf>, Vec<PathBuf>) =
            paths.into_iter().partition(|p| !p.exists());

        let paths = filter_ignores(paths);

        let mut rule = Rule {
            ignores: HashSet::new(),
            assets: HashSet::new(),
            hooks: HashSet::new(),
            actions: Vec::new(),
            layouts: HashSet::new(),
            partials: HashSet::new(),
            templates: HashSet::new(),
            deletions: deletions.into_iter().collect::<HashSet<_>>(),
        };

        let ext = project.config.engine().extension().to_string();

        let config_file = project.config.file();
        let cfg_file = config_file.canonicalize()?;

        let hooks = if let Some(ref hooks) = project.config.hooks {
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

        let build_output = canonical(project.options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper

        let assets = canonical(project.options.get_assets_path());
        let partials = canonical(project.options.get_partials_path());
        let layouts = canonical(project.options.get_layouts_path());

        // FIXME: this does not respect when data sources have a `from` directory configured
        let generators = canonical(project.options.get_data_sources_path());

        let generator_paths: Vec<PathBuf> = project
            .datasource
            .map
            .values()
            .map(|g| canonical(g.source.clone()))
            .collect::<Vec<_>>();

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
                    } else if is_template {
                        rule.templates.insert(path);

                    // Because it is valid to configure source = "."
                    // in site.toml we need to detect build output and
                    // ensure we ignore those files
                    } else if path.starts_with(&build_output) {
                        rule.ignores.insert(path);
                    } else if path.starts_with(&assets) {
                        rule.assets.insert(path);
                    } else if path.starts_with(&generators) {
                        for p in &generator_paths {
                            let documents =
                                collections::get_datasource_documents_path(p);
                            if path.starts_with(documents) {
                                rule.actions
                                    .push(Action::CollectionsDocument(path));
                                break;
                            }
                        }
                    } else {
                        let file_type = project.options.get_type(&path);
                        match file_type {
                            FileType::Unknown => {
                                rule.actions.push(Action::File(path));
                            }
                            _ => {
                                rule.actions.push(Action::Page(path));
                            }
                        }
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }

        Ok(rule)
    }

    pub async fn invalidate(&mut self, rule: &Rule) -> Result<()> {
        Ok(self.updater.invalidate(rule).await?)
    }
}
