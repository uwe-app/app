use std::path::{Path, PathBuf};

use crossbeam::channel;
use ignore::{WalkBuilder, WalkState};

pub(crate) fn find<P: AsRef<Path>, F>(dir: P, filter: F) -> Vec<PathBuf>
where
    F: Fn(&PathBuf) -> bool + Sync,
{
    let (tx, rx) = channel::unbounded();

    WalkBuilder::new(dir)
        .follow_links(true)
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
