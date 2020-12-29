use std::path::PathBuf;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;

use log::{info, warn};

use crate::{project::Project, utils::extract_locale, Result};

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

    pub(crate) fn remove(&mut self, paths: &HashSet<PathBuf>) -> Result<()> {
        let project_path = self.project.config.project().to_path_buf();
        let cwd = std::env::current_dir()?;

        for path in paths {
            // NOTE: cannot use relative_to() when files have been deleted!
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
