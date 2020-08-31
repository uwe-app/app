use std::path::Path;

use config::Config;
use crate::{Error, Result};

#[derive(Debug)]
pub enum ProjectEntry {
    One(Config),
    Many(Vec<Config>),
}

#[derive(Debug, Default)]
pub struct Workspace {
    pub projects: Vec<ProjectEntry>,
}

impl Workspace {
    pub fn flatten(self) -> Vec<Config> {
        let configs: Vec<Config> = Vec::new();
        self.projects.into_iter().fold(configs, |mut acc, p| {
            match p {
                ProjectEntry::One(c) => acc.push(c),
                ProjectEntry::Many(mut list) => acc.append(&mut list),
            }
            acc
        })
    }
}

pub fn find<P: AsRef<Path>>(dir: P, walk_ancestors: bool) -> Result<Workspace> {
    let mut workspace: Workspace = Default::default();
    let cfg = Config::load(dir.as_ref(), walk_ancestors)?;

    if let Some(ref projects) = &cfg.workspace {
        let mut members: Vec<Config> = Vec::new();
        for space in &projects.members {
            let mut root = cfg.get_project();
            root.push(space);
            if !root.exists() || !root.is_dir() {
                return Err(Error::NotDirectory(root));
            }

            let cfg = Config::load(&root, false)?;
            if cfg.workspace.is_some() {
                return Err(Error::NoNestedWorkspace(root))
            }

            members.push(cfg);
        }

        workspace.projects.push(ProjectEntry::Many(members));
    } else {
        workspace.projects.push(ProjectEntry::One(cfg));
    }

    Ok(workspace)
}
