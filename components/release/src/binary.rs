use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;

use log::info;

use crate::Result;

#[cfg(target_os = "windows")]
pub(crate) fn permissions(binaries: &HashMap<String, PathBuf>) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(crate) fn permissions(binaries: &HashMap<String, PathBuf>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for (_name, src) in binaries {
        let metadata = src.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&src, permissions)?;
    }
    Ok(())
}

pub(crate) fn symlink(binaries: &HashMap<String, PathBuf>) -> Result<()> {
    let bin_dir = cache::get_bin_dir()?;
    for (name, src) in binaries {
        let dest = bin_dir.join(name); 
        if dest.exists() {
            fs::remove_file(&dest)?;
        }
        info!("Link {} -> {}", src.display(), dest.display());
        utils::symlink::soft(src, &dest)?;
    }
    Ok(())
}
