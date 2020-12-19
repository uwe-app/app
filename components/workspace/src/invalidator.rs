use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;

use collections::{self, DataSourceMap};
use config::{hook::HookConfig, FileType};

use crate::{renderer::RenderOptions, Error, Project, Result};

/*
 *  Invalidation rules.
 *
 *  - BuildOutput: directory is ignored.
 *  - SiteConfig: (site.toml) is ignored.
 *  - Partial: trigger a build of all pages.
 *  - Layout: trigger a build of all pages.
 *  - Asset: trigger a full build.
 *  - Page: rebuild the page.
 *  - File: copy the file to build.
 *  - Resource: ignored as they are symbolically linked.
 *  - Hook: execute the hook.
 *  - DataSourceConfig: TODO.
 *  - DataSourceDocument: TODO.
 */
#[derive(Debug)]
pub enum Action {
    // Because it is valid to configure source = "."
    // in site.toml we need to detect build output and
    // ensure we ignore those files
    BuildOutput(PathBuf),

    // This is not used at the moment but we detect it;
    // it corresponds to the site.toml file.
    SiteConfig(PathBuf),

    Partial(PathBuf),
    Layout(PathBuf),
    Asset(PathBuf),
    Page(PathBuf),
    File(PathBuf),
    Hook(HookConfig, PathBuf),
    DataSourceConfig(PathBuf),
    DataSourceDocument(PathBuf),
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
    // Actions that are ignored but we track for debugging
    ignores: Vec<Action>,
    // Hooks are a special case so we store them separately
    hooks: Vec<Action>,
    // List of actions corresponding to the files that changed
    actions: Vec<Action>,
    // List of paths that do not exist anymore
    deletions: Vec<PathBuf>,
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
            if let Some(parent) = path.parent()  {
                for entry in WalkBuilder::new(parent)
                    .max_depth(Some(1))
                    .filter_entry(move |entry| {
                        entry.path() == path
                    }).build() {
                    match entry {
                        Ok(entry) => {
                            if entry.path().is_file() {
                                results.push(entry.path().to_path_buf())
                            }
                        },
                        _ => {},
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
            ignores: Vec::new(),
            hooks: Vec::new(),
            actions: Vec::new(),
            deletions: Vec::new(),
        };

        let config_file = self.project.config.file.as_ref().unwrap();
        let cfg_file = config_file.canonicalize()?;

        let hooks = if let Some(ref hooks) = self.project.config.hooks {
            hooks
                .iter()
                .filter(|h| {
                    h.has_matchers() && h.watch.is_some() && h.watch.unwrap()
                })
                .map(|h| (h, h.filter(&paths)))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        let build_output = self.canonical(self.project.options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper

        let assets = self.canonical(self.project.options.get_assets_path());
        let partials = self.canonical(self.project.options.get_partials_path());

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

        // TODO: recognise custom layouts (layout = )

        'paths: for path in paths {
            if !path.exists() {
                rule.deletions.push(path);
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
                                rule.hooks.push(Action::Hook(
                                    (*hook).clone(),
                                    f.to_path_buf(),
                                ));
                                continue 'paths;
                            }
                        }
                    }

                    if path == cfg_file {
                        rule.ignores.push(Action::SiteConfig(path));
                    //} else if path == layout_file {
                    //rule.strategy = Strategy::Page;
                    //rule.ignores.push(Action::Layout(path));
                    } else if path.starts_with(&build_output) {
                        rule.ignores.push(Action::BuildOutput(path));
                    } else if path.starts_with(&assets) {
                        rule.strategy = Strategy::Full;
                        rule.actions.push(Action::Asset(path));
                    } else if path.starts_with(&partials) {
                        rule.strategy = Strategy::Page;
                        rule.ignores.push(Action::Partial(path));
                    } else if path.starts_with(&generators) {
                        for p in &generator_paths {
                            let cfg =
                                DataSourceMap::get_datasource_config_path(p);
                            let documents =
                                collections::get_datasource_documents_path(p);
                            if path == cfg {
                                rule.actions
                                    .push(Action::DataSourceConfig(path));
                                break;
                            } else if path.starts_with(documents) {
                                rule.actions
                                    .push(Action::DataSourceDocument(path));
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

    fn remove(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
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

        for hook in &rule.hooks {
            if let Action::Hook(hook, _file) = hook {
                self.project.run_hook(hook).await?;
            }
        }

        match rule.strategy {
            Strategy::Full | Strategy::Page => {
                // TODO: handle updating search index
                //let _parse_data =
                //self.builder.build(&self.parser, target).await?;
                self.render().await?;
            }
            _ => {
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
