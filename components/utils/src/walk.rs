use std::path::{Path, PathBuf};
use std::{
    fs::{self, File},
    io,
};

use crossbeam::channel;
use ignore::{WalkBuilder, WalkState};

use crate::Result;

/// Recursively walk a folder and gather the files.
pub fn find<P: AsRef<Path>, F>(dir: P, filter: F) -> Vec<PathBuf>
where
    F: Fn(&PathBuf) -> bool + Sync,
{
    let (tx, rx) = channel::unbounded();

    WalkBuilder::new(dir)
        .follow_links(true)
        // NOTE: we need hidden files for .gitignore when
        // NOTE: creating new projects with `init`
        .hidden(false)
        .build_parallel()
        .run(|| {
            Box::new(|result| {
                if let Ok(entry) = result {
                    let path = entry.path().to_path_buf();
                    if filter(&path) {
                        let _ = tx.send(path);
                    }
                }
                WalkState::Continue
            })
        });

    drop(tx);

    rx.iter().collect()
}

/// Copy all the files returned by a walk and write to the destination
/// directory. Any existing files will be overwritten so it is the caller's
/// responsibility to detect if a target already exists.
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>, F>(
    from: P,
    to: Q,
    filter: F,
) -> Result<()>
where
    F: Fn(&PathBuf) -> bool + Sync,
{
    let contents = find(from.as_ref(), filter);
    for p in contents {
        let rel = p.strip_prefix(from.as_ref())?;
        let dest = to.as_ref().join(rel);
        if p.is_dir() {
            fs::create_dir_all(dest)?;
        } else {
            let mut source = File::open(p)?;
            let mut file = if dest.exists() {
                File::open(&dest)?
            } else {
                File::create(&dest)?
            };
            io::copy(&mut source, &mut file)?;
        }
    }
    Ok(())
}
