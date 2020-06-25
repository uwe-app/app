use std::path::Path;

use crate::{Error, Result};
use crate::config::Config;

use super::Workspace;

pub fn find<P: AsRef<Path>>(
    dir: P,
    walk_ancestors: bool,
    spaces: &mut Vec<Workspace>) -> Result<()> {

    let project = dir.as_ref();
    let cfg = Config::load(&project, walk_ancestors)?;

    if let Some(ref workspaces) = &cfg.workspace {
        for space in &workspaces.members {
            let mut root = cfg.get_project();
            root.push(space);
            if !root.exists() || root.is_file() {
                return Err(Error::new(format!("Workspace must be a directory")));
            }

            // Recursive so that workspaces can reference
            // other workspaces if they need to
            find(root, false, spaces)?;
        }
    } else {
        spaces.push(Workspace::new(cfg));
    }

    Ok(())
}
