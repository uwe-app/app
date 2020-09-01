use std::path::Path;

use config::{Config, ProfileSettings, RuntimeOptions};
use locale::Locales;

use crate::{Error, Result};

#[derive(Debug)]
pub enum ProjectEntry {
    One(Entry),
    Many(Vec<Entry>),
}

#[derive(Debug)]
pub struct Entry {
    pub config: Config,
}

impl Entry {
    pub fn map_options(self, args: &ProfileSettings) -> Result<EntryOptions> {
        let options = crate::options::prepare(&self.config, args)?;
        Ok(EntryOptions {
            config: self.config,
            locales: Default::default(),
            options,
        })
    }
}

#[derive(Debug)]
pub struct EntryOptions {
    pub config: Config,
    pub options: RuntimeOptions,
    pub locales: Locales,
}

impl EntryOptions {
    pub fn load_locales(&mut self) -> Result<()> {
        self.locales.load(&self.config, &self.options)?;
        let locale_map = self.locales.get_locale_map(&self.config.lang)?;
        self.options.locales = locale_map;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Workspace {
    pub projects: Vec<ProjectEntry>,
}

impl Workspace {

    pub fn is_empty(&self) -> bool {
        self.projects.is_empty() 
    }

    pub fn has_multiple_projects(&self) -> bool {
        if self.projects.len() > 1 { return true };
        if self.projects.len() == 1 {
            return match self.projects.first().unwrap() {
                ProjectEntry::Many(_) => true, 
                ProjectEntry::One(_) => false, 
            }
        };
        false
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &Entry> {
        self.projects
            .iter()
            .map(|e| {
                match e {
                    ProjectEntry::One(c) => vec![c],
                    ProjectEntry::Many(c) => c.iter().collect(),
                } 
            })
            .flatten()
            .collect::<Vec<&Entry>>()
            .into_iter()
    }

    pub fn iter_mut(&mut self) -> impl IntoIterator<Item = &mut Entry> {
        self.projects
            .iter_mut()
            .map(|e| {
                match e {
                    ProjectEntry::One(c) => vec![c],
                    ProjectEntry::Many(c) => c.iter_mut().collect(),
                } 
            })
            .flatten()
            .collect::<Vec<&mut Entry>>()
            .into_iter()
    }

}

pub fn load<P: AsRef<Path>>(dir: P, walk_ancestors: bool) -> Result<Workspace> {
    let mut workspace: Workspace = Default::default();
    let config = Config::load(dir.as_ref(), walk_ancestors)?;

    if let Some(ref projects) = &config.workspace {
        let mut members: Vec<Entry> = Vec::new();
        for space in &projects.members {
            let mut root = config.get_project();
            root.push(space);
            if !root.exists() || !root.is_dir() {
                return Err(Error::NotDirectory(root));
            }

            let config = Config::load(&root, false)?;
            if config.workspace.is_some() {
                return Err(Error::NoNestedWorkspace(root))
            }

            members.push(Entry { config });
        }

        workspace.projects.push(ProjectEntry::Many(members));
    } else {
        workspace.projects.push(ProjectEntry::One(Entry{ config }));
    }

    Ok(workspace)
}
