use std::path::PathBuf;

use futures::TryFutureExt;

use async_recursion::async_recursion;

use crate::{Error, Result, PackageReader};
use config::{DependencyTarget, Dependency, DependencyMap, Plugin, PLUGIN};

pub async fn read_path(file: &PathBuf) -> Result<Plugin> {
    let parent = file.parent()
        .expect("Plugin file must have parent directory")
        .to_path_buf();
    let plugin_content = utils::fs::read_string(file)?;
    let mut plugin: Plugin = toml::from_str(&plugin_content)?;
    plugin.base = parent;
    Ok(plugin)
}

pub async fn read(path: &PathBuf) -> Result<Plugin> {
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

async fn load(dep: &Dependency) -> Result<Plugin> {
    match dep.target {
        DependencyTarget::File {ref path} => {
            return Ok(read(&path).await?)
        }
        DependencyTarget::Archive {ref archive} => {
            let dir = tempfile::tempdir()?;

            // Must go into the tempdir so it is not 
            // automatically cleaned up before we 
            // are done with it.
            let path = dir.into_path();
            let reader = PackageReader::new(archive.clone(), None)
                .destination(&path, true)?
                .xz()
                .and_then(|b| b.tar())
                .await?;

            let (target, _digest, plugin) = reader.into_inner();

            println!("Archive plugin {:#?}", &plugin);
            println!("Archive plugin target {:#?}", &target);

            // Clean up the temp dir
            println!("Removing the temp archive {}", target.display());
            std::fs::remove_dir_all(target)?;
            
            todo!()
        }
    }
}

#[async_recursion]
pub async fn solve(
    input: DependencyMap,
    output: &mut DependencyMap,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
        dep.name = Some(name.clone());
        let mut plugin = load(&dep).await?;

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
