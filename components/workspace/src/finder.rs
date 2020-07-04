use std::path::Path;

use config::Config;

use crate::{Error, Result};

pub fn find<P: AsRef<Path>>(dir: P, walk_ancestors: bool, spaces: &mut Vec<Config>) -> Result<()> {
    let project = dir.as_ref();
    let cfg = Config::load(&project, walk_ancestors)?;

    if let Some(ref workspaces) = &cfg.workspace {
        for space in &workspaces.members {
            let mut root = cfg.get_project();
            root.push(space);
            if !root.exists() || !root.is_dir() {
                return Err(Error::NotDirectory(root));
            }

            // Recursive so that workspaces can reference
            // other workspaces if they need to
            find(root, false, spaces)?;
        }
    } else {
        spaces.push(cfg);
    }

    Ok(())
}
