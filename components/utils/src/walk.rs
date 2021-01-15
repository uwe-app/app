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
        // WARN: Better not to read from parents otherwise
        // WARN: copying from blueprint plugins will
        // WARN: read the .gitignore in the releases
        // WARN: directory and we don't want that.
        .parents(false)
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

/// Find files that are direct descendeants of a folder.
///
/// This version does not use a walk builder therefore is not subject
/// to .gitignore or other ignore rules.
pub fn read_dir<P: AsRef<Path>, F>(parent: P, filter: F) -> Result<Vec<PathBuf>>
where
    F: Fn(&PathBuf) -> bool + Sync,
{
    let mut files = Vec::new();
    for entry in fs::read_dir(parent.as_ref())? {
        let entry = entry?;
        let buf = entry.path().to_path_buf();
        if filter(&buf) {
            files.push(buf);
        }
    }
    Ok(files)
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
