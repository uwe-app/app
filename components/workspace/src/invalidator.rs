use std::path::Path;
use std::path::PathBuf;
use std::collections::HashSet;
use std::ffi::OsStr;

use ignore::WalkBuilder;

//use collections;
use config::{hook::HookConfig, FileType};

use crate::{renderer::RenderOptions, Error, Project, Result};

/*
 *  Invalidation rules.
 *
 *  - Asset: trigger a full build.
 *  - Page: rebuild the page.
 *  - File: copy the file to build.
 *  - CollectionsDocument: TODO.
 */
#[derive(Debug)]
pub enum Action {
    Asset(PathBuf),
    Page(PathBuf),
    File(PathBuf),
    CollectionsDocument(PathBuf),
}

#[derive(Debug)]
pub enum Strategy {
    // Trigger a full rebuild
    Full,
    // Trigger a build of all pages
    Page,
    // Iterate and process each action
    Mixed,
}

#[derive(Debug)]
pub struct Rule {
    // Notify connected websocket clients, always true for now
    pub notify: bool,
    // Reload the site data source
    reload: bool,
    // Build strategy
    strategy: Strategy,
    // Paths that are ignored but we track for debugging
    ignores: HashSet<PathBuf>,
    // Hooks are a special case so we store them separately
    hooks: HashSet<(HookConfig, PathBuf)>,
    // Layouts need special handling so that referenced pages
    // are also rendered
    layouts: HashSet<PathBuf>,
    // Partials should be re-compiled but currently we don't
    // know which files are dependent upon partials
    partials: HashSet<PathBuf>,
    // Templates can be interspersed in the site folder but
    // must come after the tests for layout and partials and
    // behave like partials in that they are re-compiled but
    // we don't know which files reference each template
    templates: HashSet<PathBuf>,
    // List of actions corresponding to the files that changed
    actions: Vec<Action>,
    // List of paths that do not exist anymore
    deletions: HashSet<PathBuf>,
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
    project: &'a mut Project,
}

impl<'a> Invalidator<'a> {
    pub fn new(project: &'a mut Project) -> Self {
        Self { project }
    }

    /// Try to find a page href from an invalidation path.
    ///
    /// Used by the live reload functionality to notify the browser
    /// it should navigate to the last edited page (follow-edits).
    pub fn find_page_href(&self, path: &PathBuf) -> Option<String> {
        if self.project.config.livereload().follow_edits() {
            if let Ok(file) = self.project.options.relative_to(
                path,
                &self.project.options.source,
                &self.project.options.source,
            ) {
                for renderer in self.project.renderers.iter() {
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

    fn canonical<P: AsRef<Path>>(&self, src: P) -> PathBuf {
        let file = src.as_ref().to_path_buf();
        if file.exists() {
            if let Ok(canonical) = file.canonicalize() {
                return canonical;
            }
        }
        file
    }

    /// Walk the parent directory so we can determine if a path
    /// should be ignored using the standard .gitignore and .ignore
    /// file comparisons.
    ///
    /// This is inefficient because we have to walk all the entries
    /// in the parent directory to determine if a file should be
    /// ignored.
    ///
    /// Ideally we could do this at a lower-level but the `ignore`
    /// crate does not expose the `dir` module so we would need to
    /// reproduce all of that functionality.
    fn filter_ignores(&self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut results: Vec<PathBuf> = Vec::new();
        for path in paths {
            if let Some(parent) = path.parent() {
                for entry in WalkBuilder::new(parent)
                    .max_depth(Some(1))
                    .filter_entry(move |entry| entry.path() == path)
                    .build()
                {
                    match entry {
                        Ok(entry) => {
                            if entry.path().is_file() {
                                results.push(entry.path().to_path_buf())
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        results
    }

    pub fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Rule> {
        let paths = self.filter_ignores(paths);

        let mut rule = Rule {
            notify: true,
            reload: false,
            strategy: Strategy::Mixed,
            ignores: HashSet::new(),
            hooks: HashSet::new(),
            actions: Vec::new(),
            layouts: HashSet::new(),
            partials: HashSet::new(),
            templates: HashSet::new(),
            deletions: HashSet::new(),
        };

        let ext = self.project.config.engine().extension().to_string();

        let config_file = self.project.config.file.as_ref().unwrap();
        let cfg_file = config_file.canonicalize()?;

        let hooks = if let Some(ref hooks) = self.project.config.hooks {
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

        let build_output = self.canonical(self.project.options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper

        let assets = self.canonical(self.project.options.get_assets_path());
        let partials = self.canonical(self.project.options.get_partials_path());
        let layouts = self.canonical(self.project.options.get_layouts_path());

        // FIXME: this does not respect when data sources have a `from` directory configured
        let generators =
            self.canonical(self.project.options.get_data_sources_path());

        let generator_paths: Vec<PathBuf> = self
            .project
            .datasource
            .map
            .values()
            .map(|g| self.canonical(g.source.clone()))
            .collect::<Vec<_>>();

        'paths: for path in paths {
            if !path.exists() {
                rule.deletions.insert(path);
                continue;
            }

            match path.canonicalize() {
                Ok(path) => {
                    // NOTE: must test for hooks first as they can
                    // NOTE: point anywhere in the source directory
                    // NOTE: and should take precedence
                    for (hook, files) in hooks.iter() {
                        for f in files.iter() {
                            if &path == f {
                                rule.hooks.insert((
                                    (*hook).clone(),
                                    f.to_path_buf(),
                                ));
                                continue 'paths;
                            }
                        }
                    }

                    let is_template = if let Some(extension) = path.extension() {
                        extension == OsStr::new(&ext)
                    } else { false };

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
                        rule.strategy = Strategy::Full;
                        rule.actions.push(Action::Asset(path));
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
                        let file_type = self.project.options.get_type(&path);
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

    fn remove(&mut self, paths: &HashSet<PathBuf>) -> Result<()> {
        let project = self.project.config.project().to_path_buf();
        let cwd = std::env::current_dir()?;

        for path in paths {
            // NOTE: cannot use relative_to() when files have been deleted!
            let relative = if project.is_absolute() {
                path.strip_prefix(&project).unwrap_or(path).to_path_buf()
            } else {
                path.strip_prefix(&cwd).unwrap_or(path).to_path_buf()
            };

            let (lang, path) = self.extract_locale(&relative);
            self.project.remove_file(&path, lang)?;
        }
        Ok(())
    }

    pub async fn invalidate(&mut self, rule: &Rule) -> Result<()> {
        // Reload the config data!
        if rule.reload {
            // FIXME: to restore this we need to reload and parse the configuration!
            //
            //if let Err(e) = loader::reload(config, options) {
            //error!("{}", e);
            //}
        }

        // Remove deleted files.
        if !rule.deletions.is_empty() {
            self.remove(&rule.deletions)?;
        }

        for (hook, file) in &rule.hooks {
            self.project.run_hook(hook, Some(file)).await?;
        }

        match rule.strategy {
            Strategy::Full | Strategy::Page => {
                // TODO: handle updating search index
                //let _parse_data =
                //self.builder.build(&self.parser, target).await?;
                self.render().await?;
            }
            _ => {

                if !rule.templates.is_empty() {
                    self.project.update_templates(&rule.templates).await?;
                }

                if !rule.partials.is_empty() {
                    self.project.update_partials(&rule.partials).await?;
                }

                if !rule.layouts.is_empty() {
                    self.project.update_layouts(&rule.layouts).await?;
                }

                for action in &rule.actions {
                    match action {
                        Action::Page(path) | Action::File(path) => {
                            // Make the path relative to the project source
                            // as the notify crate gives us an absolute path
                            let file = self.project.options.relative_to(
                                path,
                                &self.project.options.source,
                                &self.project.options.source,
                            )?;

                            self.one(&file).await?;
                        }
                        _ => {
                            return Err(Error::InvalidationActionNotHandled);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Render the entire project.
    async fn render(&mut self) -> Result<()> {
        self.project.render(Default::default()).await?;
        Ok(())
    }

    /// Render a single file using the appropriate locale-specific renderer.
    async fn one(&mut self, file: &PathBuf) -> Result<()> {
        // Raw source files might be localized variants
        // we need to strip the locale identifier from the
        // file path before compiling
        let (lang, file) = self.extract_locale(&file);
        let lang: &str = if let Some(ref lang) = lang {
            lang.as_str()
        } else {
            &self.project.config.lang
        };

        let options = RenderOptions::new_file_lang(
            file,
            lang.to_string(),
            true,
            false,
            false,
        );

        self.project.render(options).await?;

        Ok(())
    }

    /// Extract locale identifier from a file name when possible.
    fn extract_locale(&self, file: &PathBuf) -> (Option<String>, PathBuf) {
        let languages = self.project.locales.languages().alternate();
        if let Some((lang, path)) =
            collator::get_locale_file_info(&file.as_path(), &languages)
        {
            return (Some(lang), path);
        }
        (None, file.to_path_buf())
    }
}
