use std::path::{Path, PathBuf};

use config::{Plugin, PLUGIN, dependency::DependencyTarget};

use crate::{Error, Result};

static NORMALIZED_HEADER: &str = "\
# Automatically generated plugin file, see plugin.orig.toml for the raw content.
#
# Generating an archive of a plugin indicates that it should be portable; this 
# version of the plugin has removed paths and archive dependencies such that all
# dependencies should be resolved from a remote registry or repository.
";

async fn normalize_plugin<P: AsRef<Path>>(file: P) -> Result<(Plugin, Plugin)> {
    let original = read_path(file).await?;
    let mut plugin = original.clone();
    if let Some(ref mut deps) = plugin.dependencies {
        for (_, dep) in deps.iter_mut() {
            if let Some(ref target) = dep.target {
                match target {
                    DependencyTarget::File {..} | DependencyTarget::Archive {..} => {
                        dep.target = None;
                    }
                }
            }
        }
    }
    Ok((original, plugin))
}

/// Create a normalized portable representation of a plugin suitable for 
/// packaging to an archive.
pub async fn normalize<P: AsRef<Path>>(file: P) -> Result<(String, String)> {
    let (original, plugin) = normalize_plugin(&file).await?;
    let original = utils::fs::read_string(file)?;
    let plugin = &toml::to_string(&plugin)?;
    let mut out = String::new();
    out.push_str(NORMALIZED_HEADER);
    out.push_str(&plugin);
    Ok((original, out))
}

/// Compute plugin information by convention from the file system.
async fn compute<P: AsRef<Path>>(file: P) -> Result<()> {
    let original = read_path(file).await?;
    //let source = original.base;
    Ok(())
}

pub async fn read_path<P: AsRef<Path>>(file: P) -> Result<Plugin> {
    let file = file.as_ref();
    let parent = file
        .parent()
        .expect("Plugin file must have parent directory")
        .to_path_buf();
    let plugin_content = utils::fs::read_string(file)?;
    let mut plugin: Plugin = toml::from_str(&plugin_content)?;
    plugin.set_base(parent);
    Ok(plugin)
}

pub async fn read<P: AsRef<Path>>(path: P) -> Result<Plugin> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(Error::BadPluginPath(path.to_path_buf()));
    }

    let file = if path.ends_with(PLUGIN) {
        path.to_path_buf()
    } else {
        path.join(PLUGIN)
    };

    if !file.exists() || !file.is_file() {
        return Err(Error::BadPluginFile(file));
    }

    read_path(&file).await
}
