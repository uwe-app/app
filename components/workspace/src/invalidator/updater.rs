use std::path::PathBuf;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;

use log::{info, warn};

use config::{hook::HookConfig};

use crate::{
    project::Project,
    renderer::RenderOptions,
    Result,
};

use super::{
    Kind,
    Rule,
    utils::{extract_locale, relative_to}, 
};

pub struct Updater<'a> {
    project: &'a mut Project,
}

impl<'a> Updater<'a> {
    pub fn new(project: &'a mut Project) -> Self {
        Self {project}
    }

    pub fn project(&mut self) -> &mut Project {
        self.project
    }

    pub(crate) fn update_deletions(&mut self, paths: &HashSet<PathBuf>) -> Result<()> {
        let project_path = self.project.config.project().to_path_buf();
        let cwd = std::env::current_dir()?;

        for path in paths {
            // NOTE: cannot use relative_to() when files have been deleted
            // NOTE: because is call canonicalize() which can fail
            let relative = if project_path.is_absolute() {
                path.strip_prefix(&project_path).unwrap_or(path).to_path_buf()
            } else {
                path.strip_prefix(&cwd).unwrap_or(path).to_path_buf()
            };

            let (lang, path) = extract_locale(&relative, self.project.locales.languages().alternate());
            self.remove_file(&path, lang)?;
        }
        Ok(())
    }

    /// Execute hooks that have changed.
    pub(crate) async fn update_hooks(&mut self, hooks: &HashSet<(HookConfig, PathBuf)>) -> Result<()> {
        for (hook, file) in hooks {
            self.project.run_hook(hook, Some(file)).await?;
        }
        Ok(())
    }

    /// Update templates.
    pub(crate) async fn update_templates(
        &mut self,
        templates: &HashSet<PathBuf>,
    ) -> Result<()> {
        for template in templates {
            let name = template.to_string_lossy();
            if template.exists() {
                info!("Render template {}", &name);
                for parser in self.project.parsers_mut().iter_mut() {
                    // Re-compile the template
                    parser.load(template)?;
                }
            } else {
                info!("Delete template {}", &name);
                for parser in self.project.parsers_mut().iter_mut() {
                    // Remove the template from the parser
                    parser.remove(&name);
                }
            }
        }

        Ok(())
    }

    /// Update partials.
    pub(crate) async fn update_partials(
        &mut self,
        partials: &HashSet<PathBuf>,
    ) -> Result<()> {
        let partials: Vec<(String, &PathBuf)> = partials
            .iter()
            .map(|layout| {
                let name =
                    layout.file_stem().unwrap().to_string_lossy().into_owned();
                (name, layout)
            })
            .collect();

        for (name, partial) in partials {
            if partial.exists() {
                info!("Render partial {}", &name);
                for parser in self.project.parsers_mut().iter_mut() {
                    // Re-compile the template
                    parser.add(name.to_string(), partial)?;
                }
            } else {
                info!("Delete partial {}", &name);
                for parser in self.project.parsers_mut().iter_mut() {
                    // Remove the partial from the parser
                    parser.remove(&name);
                }
            }
        }

        Ok(())
    }

    /// Update layouts and render any pages referenced by the layouts.
    pub(crate) async fn update_layouts(
        &mut self,
        layouts: &HashSet<PathBuf>,
    ) -> Result<()> {
        // List of pages to render
        let mut render_pages: HashSet<(String, PathBuf)> = HashSet::new();

        let layouts: Vec<(String, &PathBuf)> = layouts
            .iter()
            .map(|layout| {
                let name =
                    layout.file_stem().unwrap().to_string_lossy().into_owned();
                (name, layout)
            })
            .collect();

        // TODO: handle new layouts
        // TODO: handle deleted layouts

        for (name, layout) in layouts {
            if layout.exists() {
                info!("Render layout {}", &name);
                for (parser, renderer) in self.project.iter_mut()
                {
                    // Re-compile the template
                    parser.add(name.to_string(), layout)?;

                    // Collect pages that match the layout name
                    // so they can be rendered
                    let collation =
                        &*renderer.info.context.collation.read().unwrap();
                    let fallback = collation.fallback.read().unwrap();
                    let lang = collation.get_lang().as_ref().to_string();
                    for (file_path, page_lock) in fallback.pages.iter() {
                        let page = page_lock.read().unwrap();
                        if !page.is_standalone() {
                            if let Some(ref layout_name) = page.layout {
                                if &name == layout_name {
                                    render_pages.insert((
                                        lang.to_string(),
                                        file_path.to_path_buf(),
                                    ));
                                }
                            }
                        }
                    }
                }
            } else {
                info!("Delete layout {}", &name);
                for (parser, renderer) in self.project.iter_mut() {
                    // Remove the layout from the parser
                    parser.remove(&name);
                    // Remove from the collated data
                    let mut collation =
                        renderer.info.context.collation.write().unwrap();
                    collation.remove_layout(&name);
                }
            }
        }

        // Render pages that require an update as they
        // reference a changed layout
        for (lang, file) in render_pages {
            let options =
                RenderOptions::new_file_lang(file, lang, true, false, false);
            self.project.render(options).await?;
        }

        Ok(())
    }

    pub async fn invalidate(&mut self, rule: &Rule) -> Result<()> {
        // Remove deleted files.
        if !rule.deletions.is_empty() {
            self.update_deletions(&rule.deletions)?;
        }

        if !rule.hooks.is_empty() {
            self.update_hooks(&rule.hooks).await?;
        }

        if !rule.templates.is_empty() {
            self.update_templates(&rule.templates).await?;
        }

        if !rule.partials.is_empty() {
            self.update_partials(&rule.partials).await?;
        }

        if !rule.layouts.is_empty() {
            self.update_layouts(&rule.layouts).await?;
        }

        for action in &rule.actions {
            match action {
                Kind::Page(path) | Kind::File(path) => {
                    // Make the path relative to the project source
                    // as the notify crate gives us an absolute path
                    let source = self.project.options.source.clone();
                    let file = relative_to(path, &source, &source)?;

                    self.one(&file).await?;
                }
            }
        }
        Ok(())
    }

    /// Render a single file using the appropriate locale-specific renderer.
    async fn one(&mut self, file: &PathBuf) -> Result<()> {
        // Raw source files might be localized variants
        // we need to strip the locale identifier from the
        // file path before compiling
        let (lang, file) =
            extract_locale(&file, self.project.locales.languages().alternate());
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

    /// Helper function to remove a file from the collation.
    fn remove_file(
        &mut self,
        path: &PathBuf,
        mut lang: Option<String>,
    ) -> Result<()> {
        let lang = if let Some(lang) = lang.take() {
            lang
        } else {
            self.project.config().lang.clone()
        };

        // Find the correct renderer so we access the collation
        // for the language
        if let Some(renderer) = self.project.renderers().iter().find(|r| {
            let collation = r.info.context.collation.read().unwrap();
            let locale = collation.locale.read().unwrap();
            locale.lang == lang
        }) {
            info!("Delete {} -> {}", &lang, path.display());

            // Get the href we can use to get the build product location
            // for deleting from the build directory
            let mut collation =
                renderer.info.context.collation.write().unwrap();

            // Must get the target href before we remove
            // from the collation
            let href = if let Some(href) = collation.get_link_href(path) {
                Some(href.as_ref().to_string())
            } else {
                None
            };

            // Remove from the internal data structure
            collation.remove_file(path, self.project.options());

            // Now try to remove the build product
            if let Some(ref href) = href {
                let build_file = self.project.options().base.join(
                    utils::url::to_path_separator(href.trim_start_matches("/")),
                );

                if build_file.exists() {
                    info!("Remove {}", build_file.display());

                    if let Err(e) = fs::remove_file(&build_file) {
                        warn!(
                            "Failed to remove build file {}: {}",
                            build_file.display(),
                            e
                        );
                    }

                    // If we have an `index.html` file then we might
                    // have an empty directory for the parent, let's
                    // try to clean it up too.
                    if let Some(file_name) = build_file.file_name() {
                        if file_name == OsStr::new(config::INDEX_HTML) {
                            if let Some(parent) = build_file.parent() {
                                // The call to remove_dir() will fail if
                                // the directory is not empty
                                let _ = fs::remove_dir(parent);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

}
