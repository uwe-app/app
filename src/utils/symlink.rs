use std::path::Path;

use crate::Error;

#[cfg(windows)]
pub fn soft<P: AsRef<Path>>(source: P, target: P) -> Result<(), Error> {
    let path = source.as_ref();
    if path.is_dir() {
        return std::os::windows::fs::symlink_dir(source, target)
            .map_err(Error::from);
    } else if path.is_file() {
        return std::os::windows::fs::symlink_file(source, target)
            .map_err(Error::from);
    }
    Ok(())
}

#[cfg(unix)]
pub fn soft<P: AsRef<Path>>(source: P, target: P) -> Result<(), Error> {
    std::os::unix::fs::symlink(source, target).map_err(Error::from)
}
