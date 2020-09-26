use std::path::{Path, PathBuf};

use async_recursion::async_recursion;

use config::{
    dependency::{Dependency, DependencyMap, DependencyTarget},
    registry::RegistryItem,
    lock_file::LockFile,
    lock_file::LockFileEntry,
    semver::Version,
    Plugin, PLUGIN,
};

use crate::{installer, registry::RegistryAccess, Error, Result};

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

fn check_plugin(
    name: &str,
    dep: &Dependency,
    plugin: &Plugin,
    stack: &mut Vec<String>,
) -> Result<()> {
    if name != plugin.name {
        return Err(Error::PluginNameMismatch(
            name.to_string(),
            plugin.name.clone(),
        ));
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

    Ok(())
}

async fn resolve_version(
    registry: &Box<dyn RegistryAccess + Send + Sync + '_>,
    dep: &Dependency,
) -> Result<(Version, Option<RegistryItem>, Option<Plugin>)> {
    if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => {
                let plugin = read(path).await?;
                Ok((plugin.version.clone(), None, Some(plugin)))
            }
            DependencyTarget::Archive { ref archive } => todo!(),
        }
    } else {
        // Get version from registry
        let name = dep.name.as_ref().unwrap();
        let (version, package) =
            installer::resolve_package(registry, name, &dep.version).await?;
        Ok((version, Some(package), None))
    }
}

#[async_recursion]
pub async fn solve(
    registry: &Box<dyn RegistryAccess + Send + Sync + 'async_recursion>,
    input: DependencyMap,
    output: &mut DependencyMap,
    lock_file_current: &LockFile,
    lock_file_target: &mut LockFile,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
        dep.name = Some(name.clone());

        let mut entry = lock_file_current
            .package
            .iter()
            .find_map(|e| {
                if e.name == name {
                    return Some(e);
                }
                None
            })
            .map(|e| e.clone())
            .unwrap_or({
                let (version, package, mut plugin) =
                    resolve_version(registry, &dep).await?;

                if let Some(plugin) = plugin.take() {
                    check_plugin(&name, &dep, &plugin, stack)?;
                    dep.plugin = Some(plugin);
                    dep.prepare()?;
                }

                let checksum = if let Some(ref pkg) = package {
                    Some(pkg.digest.clone())
                } else {
                    None
                };

                LockFileEntry {
                    name: name.to_string(),
                    version,
                    checksum,
                    source: None,
                    dependencies: None,
                }
            });

        stack.push(name.clone());

        // FIXME: recursively resolve dependencies

        //if let Some(dependencies) = plugin.dependencies.take() {
        //let mut deps: DependencyMap = Default::default();
        //solve(
        //dependencies,
        //&mut deps,
        //lock_file_current,
        //lock_file_target,
        //stack
        //)?;
        //}


        //println!("Entry is {:#?}", entry);

        lock_file_target.package.insert(entry);

        output.items.insert(name, dep);

        //let (mut plugin, entry) = installer::install(&dep).await?;

        /*
        lock_file_target.package.insert(entry);

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
            solve(
                dependencies,
                &mut deps,
                lock_file_current,
                lock_file_target,
                stack
            ).await?;
        }

        dep.plugin = Some(plugin);
        dep.prepare()?;

        output.items.insert(name, dep);
        */
    }

    Ok(())
}

/*
#[async_recursion]
pub async fn solve(
    input: DependencyMap,
    output: &mut DependencyMap,
    lock_file_current: &LockFile,
    lock_file_target: &mut LockFile,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
        dep.name = Some(name.clone());

        let (mut plugin, entry) = installer::install(&dep).await?;

        lock_file_target.package.insert(entry);

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
            solve(
                dependencies,
                &mut deps,
                lock_file_current,
                lock_file_target,
                stack
            ).await?;
        }

        dep.plugin = Some(plugin);
        dep.prepare()?;

        output.items.insert(name, dep);
    }

    Ok(())
}
*/
