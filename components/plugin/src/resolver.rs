use std::path::PathBuf;

use futures::TryFutureExt;

use async_recursion::async_recursion;

use config::{Dependency, DependencyMap, DependencyTarget, Plugin, PLUGIN};

use crate::{Error, PackageReader, Result, registry, registry::RegistryAccess};

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
    if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => return Ok(read(&path).await?),
            DependencyTarget::Archive { ref archive } => {
                let dir = tempfile::tempdir()?;

                // FIXME: extract this to a tmp dir that can be used for the build

                // Must go into the tempdir so it is not
                // automatically cleaned up before we
                // are done with it.
                let path = dir.into_path();

                let reader = PackageReader::new(archive.clone(), None)
                    .destination(&path)?
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
    } else {
        let name = dep.name.as_ref().unwrap();
        let reg = cache::get_registry_dir()?;
        let registry = registry::RegistryFileAccess::new(reg.clone(), reg.clone())?;
        let entry = registry.entry(name).await?.ok_or_else(|| {
            Error::RegistryPackageNotFound(name.to_string()) 
        })?;

        let package = entry.find(&dep.version).ok_or_else(|| {
            Error::RegistryPackageVersionNotFound(
                name.to_string(), dep.version.to_string())
        })?;

        // TODO: 1) Check if cached version of the package exists
        // TODO: 2) Fetch, cache and unpack plugin package (verify digest!)
        // TODO: 3) Load the package plugin from the file system

        println!("Got entry {:?}", entry);
        println!("Got matched package {:?}", package);

        todo!()
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
