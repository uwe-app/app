use std::io;
use std::path::Path;

#[cfg(windows)]
pub fn soft<P: AsRef<Path>>(source: P, target: P) -> io::Result<()> {
    let path = source.as_ref();
    if path.is_dir() {
        return std::os::windows::fs::symlink_dir(source, target);
    } else if path.is_file() {
        return std::os::windows::fs::symlink_file(source, target);
    }
    Ok(())
}

#[cfg(unix)]
pub fn soft<P: AsRef<Path>>(source: P, target: P) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}
