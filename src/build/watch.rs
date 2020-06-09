#[cfg(feature = "watch")]
use std::convert::AsRef;

use notify::Watcher;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;

use crate::Error;
use log::{info, error, debug};


pub fn start<P, F>(dir: P, mut closure: F)
where
    P: AsRef<Path>,
    F: FnMut(Vec<PathBuf>, &Path) -> Result<(), Error>,
{
    use notify::DebouncedEvent::*;
    use notify::RecursiveMode::*;

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    let mut watcher = match notify::watcher(tx, Duration::from_secs(1)) {
        Ok(w) => w,
        Err(e) => {
            error!("Error while trying to watch the files:\n\n\t{:?}", e);
            std::process::exit(1)
        }
    };

    // FIXME: if --directory we must also watch data.toml and layout.hbs

    // Add the source directory to the watcher
    if let Err(e) = watcher.watch(&dir, Recursive) {
        error!("Error while watching {:?}:\n    {:?}", dir.as_ref().display(), e);
        std::process::exit(1);
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

                //event.foo();

                match event {
                    Create(path) | Write(path) | Remove(path) | Rename(_, path) => Some(path),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        if !paths.is_empty() {
            if let Err(e) = closure(paths, &dir.as_ref()) {
                error!("{}", e);
            }
        }
    }
}
