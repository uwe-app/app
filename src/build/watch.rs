#[cfg(feature = "watch")]
use std::convert::AsRef;

use notify::Watcher;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;

use crate::callback::ErrorCallback;
use crate::Error;

use log::{debug, info};

use notify::DebouncedEvent::{Create, Remove, Rename, Write};
use notify::RecursiveMode::Recursive;

pub fn start<P, F>(dir: P, error_cb: &ErrorCallback, mut closure: F) -> Result<(), Error>
where
    P: AsRef<Path>,
    F: FnMut(Vec<PathBuf>, &Path) -> Result<(), Error>,
{
    // Create a channel to receive the events.
    let (tx, rx) = channel();
    let mut watcher = match notify::watcher(tx, Duration::from_secs(1)) {
        Ok(w) => w,
        Err(e) => return Err(crate::Error::from(e)),
    };

    // FIXME: if --directory we must also watch data.toml and layout.hbs

    // Add the source directory to the watcher
    if let Err(e) = watcher.watch(&dir, Recursive) {
        return Err(crate::Error::from(e));
    };

    //let _ = watcher.watch(book.theme_dir(), Recursive);
    // Add the book.toml file to the watcher if it exists
    //let _ = watcher.watch(book.root.join("book.toml"), NonRecursive);

    info!("watch {}", dir.as_ref().display());

    loop {
        let first_event = rx.recv().unwrap();
        sleep(Duration::from_millis(50));
        let other_events = rx.try_iter();

        let all_events = std::iter::once(first_event).chain(other_events);

        let paths = all_events
            .filter_map(|event| {
                debug!("Received filesystem event: {:?}", event);
                match event {
                    Create(path) | Write(path) | Remove(path) | Rename(_, path) => Some(path),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        if !paths.is_empty() {
            if let Err(e) = closure(paths, &dir.as_ref()) {
                error_cb(e);
            }
        }
    }
}
