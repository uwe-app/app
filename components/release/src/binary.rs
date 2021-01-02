use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use log::info;

use crate::Result;

#[cfg(target_os = "windows")]
pub(crate) fn permissions(binaries: &HashMap<String, PathBuf>) -> Result<()> {
    todo!()
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
    let releases_dir = dirs::releases_dir()?;
    let bin_dir = dirs::bin_dir()?;
    for (name, src) in binaries {
        let dest = bin_dir.join(name);
        if dest.exists() {
            fs::remove_file(&dest)?;
        }

        let short_src = src.strip_prefix(&releases_dir)?;
        info!("Link {} -> {}", short_src.display(), dest.display());

        utils::symlink::soft(src, &dest)?;
    }
    Ok(())
}

pub(crate) fn symlink_names(dir: &PathBuf, names: &[&str]) -> Result<()> {
    let mut out: HashMap<String, PathBuf> = HashMap::new();
    for name in names {
        let path = dir.join(name).to_path_buf();
        if path.exists() {
            out.insert(name.to_string(), path);
        }
    }
    symlink(&out)
}
