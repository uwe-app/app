use std::path::Path;
use std::path::PathBuf;

use config::FileType;
use datasource::{self, DataSourceMap};

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
    Hook(String, PathBuf),
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
}

pub struct Invalidator<'a> {
    project: &'a mut Project,
}

impl<'a> Invalidator<'a> {
    pub fn new(project: &'a mut Project) -> Self {
        Self { project }
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

    pub fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Rule> {
        //let config = &self.project.config;
        //let options = &self.project.options;

        let mut rule = Rule {
            notify: true,
            reload: false,
            strategy: Strategy::Mixed,
            ignores: Vec::new(),
            hooks: Vec::new(),
            actions: Vec::new(),
        };

        let config_file = self.project.config.file.as_ref().unwrap();
        let cfg_file = config_file.canonicalize()?;

        let hooks = self.project.config.hooks.as_ref().unwrap();

        let build_output = self.canonical(self.project.options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper
        //let layout_file =
        //self.canonical(self.project.options.get_layout_path());

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
            match path.canonicalize() {
                Ok(path) => {
                    // NOTE: must test for hooks first as they can
                    // NOTE: point anywhere in the source directory
                    // NOTE: and should take precedence
                    for hook in hooks.iter() {
                        /*
                        if hook.source.is_some() {
                            let hook_base = self.canonical(
                                hook.get_source_path(
                                    &self.project.options.source,
                                )
                                .unwrap(),
                            );
                            if path.starts_with(hook_base) {
                                rule.hooks.push(Action::Hook(k.clone(), path));
                                continue 'paths;
                            }
                        }
                        */
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
                                datasource::get_datasource_documents_path(p);
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

    pub async fn invalidate(&mut self, rule: &Rule) -> Result<()> {
        //let livereload = context::livereload().read().unwrap();

        //let config = &self.project.config;
        //let options = &self.project.options;

        // Reload the config data!
        if rule.reload {
            // FIXME: to restore this we need to reload and parse the configuration!
            //
            //if let Err(e) = loader::reload(config, options) {
            //error!("{}", e);
            //}
        }

        // FIXME: restore hook execution on live reload?!

        /*
        for hook in &rule.hooks {
            if let Action::Hook(id, _path) = hook {
                if let Some(hook_config) = config.hook.as_ref().unwrap().get(id)
                {
                    hook::exec(Arc::clone(&self.builder.context), hook_config)?;
                }
            }
        }
        */

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

        // TODO: reload the collated page data before compiing!
        //
        let options = RenderOptions::new_file_lang(file, lang.to_string());

        self.project.render(options).await?;

        Ok(())
    }

    /// Extract locale identifier from a file name when possible.
    fn extract_locale(&self, file: &PathBuf) -> (Option<String>, PathBuf) {
        let languages = self.project.locales.languages.get_translations();
        if let Some((lang, path)) =
            collator::get_locale_file_info(&file.as_path(), &languages)
        {
            return (Some(lang), path);
        }
        (None, file.to_path_buf())
    }
}
