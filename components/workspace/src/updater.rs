use std::path::PathBuf;
use std::collections::HashSet;

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
            self.project.remove_file(&path, lang)?;
        }
        Ok(())
    }

}
