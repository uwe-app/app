use std::path::Path;

use config::Config;
use crate::{Error, Result};

#[derive(Debug)]
pub enum ProjectEntry {
    One(Config),
    Many(Vec<Config>),
}

#[derive(Debug)]
pub struct Entry {
    pub config: Config,
}

#[derive(Debug, Default)]
pub struct Workspace {
    pub projects: Vec<ProjectEntry>,

    // Iterator entry index
    entry_index: usize,
    many_index: Option<usize>,
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

    pub fn flatten(&mut self) -> impl IntoIterator<Item = &Config> {
        self.projects
            .iter()
            .map(|e| {
                match e {
                    ProjectEntry::One(c) => vec![c],
                    ProjectEntry::Many(c) => c.iter().collect(),
                } 
            })
            .flatten()
            .collect::<Vec<&Config>>()
            .into_iter()
    }

    pub fn iter(&mut self) -> impl Iterator<Item = Entry> + '_ {
        self 
    }
}

impl<'a> Iterator for &'a mut Workspace {
    type Item = Entry;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.projects.get(self.entry_index) {
            match entry {
                ProjectEntry::One(config) => {
                    self.entry_index += 1;
                    return Some(Entry{config: config.clone()})
                }
                ProjectEntry::Many(config_list) => {
                    let many_index = self.many_index.unwrap_or(0);
                    if many_index >= config_list.len() {
                        self.entry_index += 1;
                        self.many_index = None
                    } else {
                        self.many_index = Some(many_index);
                        let mi = self.many_index.as_mut().unwrap();
                        *mi += 1;
                    }
                    if let Some(config) = config_list.get(many_index) {
                        return Some(Entry{config: config.clone()})
                    }
                },
            }
        }
        self.entry_index = 0;
        self.many_index = None;
        None
    }
}

//impl Iterator for Workspace {
    //type Item = Entry;
    //fn next(&mut self) -> Option<Self::Item> {
        //if let Some(entry) = self.projects.get(self.entry_index) {
            //match entry {
                //ProjectEntry::One(config) => {
                    //self.entry_index += 1;
                    //return Some(Entry{config: config.clone()})
                //}
                //ProjectEntry::Many(config_list) => {
                    //let many_index = self.many_index.unwrap_or(0);
                    //if many_index >= config_list.len() {
                        //self.entry_index += 1;
                        //self.many_index = None
                    //} else {
                        //self.many_index = Some(many_index);
                        //*self.many_index.as_mut().unwrap() += 1;
                    //}
                    //if let Some(config) = config_list.get(many_index) {
                        //return Some(Entry{config: config.clone()})
                    //}
                //},
            //}
        //}
        //self.entry_index = 0;
        //self.many_index = None;
        //None
    //}
//}

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
