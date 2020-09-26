use std::path::{Path, PathBuf};

use async_recursion::async_recursion;

use config::{DependencyMap, Plugin, PLUGIN};

use crate::{installer, Error, Result};

pub async fn read_path(file: &PathBuf) -> Result<Plugin> {
    let parent = file
        .parent()
        .expect("Plugin file must have parent directory")
        .to_path_buf();
    let plugin_content = utils::fs::read_string(file)?;
    let mut plugin: Plugin = toml::from_str(&plugin_content)?;
    plugin.base = parent;
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

#[async_recursion]
pub async fn solve(
    input: DependencyMap,
    output: &mut DependencyMap,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
        dep.name = Some(name.clone());
        let mut plugin = installer::install(&dep).await?;

        if name != plugin.name {
            return Err(Error::PluginNameMismatch(name, plugin.name));
        }

        if stack.contains(&plugin.name) {
            return Err(Error::PluginCyclicDependency(plugin.name.clone()));
        }

        if !dep.version.matches(&plugin.version) {
            return Err(Error::PluginVersionMismatch(
                plugin.name.clone(),
                plugin.version.to_string(),
                dep.version.to_string(),
            ));
        }

        stack.push(plugin.name.clone());

        if let Some(dependencies) = plugin.dependencies.take() {
            let mut deps: DependencyMap = Default::default();
            solve(dependencies, &mut deps, stack).await?;
        }

        dep.plugin = Some(plugin);
        dep.prepare()?;

        output.items.insert(name, dep);
    }

    Ok(())
}
