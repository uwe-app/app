use std::path::Path;
use std::path::PathBuf;

use super::config::ExtensionConfig;
use super::file::FileInfo;

static INDEX_STEM: &str = "index";

fn resolve_dir_index<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> Option<PathBuf> {
    let mut buf = file.as_ref().to_path_buf();
    buf.push(INDEX_STEM);
    for ext in extensions.render.iter() {
        buf.set_extension(ext);
        if buf.exists() {
            return Some(buf);
        }
    }
    None
}

pub fn resolve_parent_index<P: AsRef<Path>>(
    file: P,
    extensions: &ExtensionConfig,
) -> Option<PathBuf> {
    if let Some(parent) = file.as_ref().parent() {
        // Not an index file so a single level is sufficient
        if !FileInfo::is_index(&file) {
            return resolve_dir_index(&parent, extensions);
        // Otherwise go back down one more level
        } else {
            if let Some(parent) = parent.parent() {
                return resolve_dir_index(&parent, extensions);
            }
        }
    }
    None
}
