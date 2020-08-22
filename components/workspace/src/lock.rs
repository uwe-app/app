use std::fs::{File, remove_file};
use std::path::PathBuf;

use fs2::FileExt;

use log::{debug, info};

use crate::Result;

pub struct LockFile<'a> {
    pub path: &'a PathBuf,
    pub file: File,
}

pub fn acquire(path: &PathBuf) -> Result<LockFile> {

    let file = if path.exists() {
        File::open(path)?
    } else {
        File::create(path)?
    };

    debug!("Lock file {}", path.display());
    if let Err(_) = file.try_lock_exclusive() {
        info!("Waiting for lock file {}", path.display());
        // Block while we wait for the lock to be released
        while let Err(_) = file.try_lock_exclusive() {}
    }
    debug!("Lock obtained {}", path.display());
    Ok(LockFile {path, file})
}

pub fn release(lock_file: LockFile) -> Result<()> {
    lock_file.file.unlock()?;
    debug!("Releasing lock {}", lock_file.path.display());
    remove_file(lock_file.path)?;
    Ok(())
}
