use std::path::Path;
use std::convert::TryInto;

use log::{info, debug};

use config::{
    Config,
    ResolvedPlugins,
    lock_file::{LockFile, LockFileEntry},
    plugin::{Plugin, dependency::Dependency},
};

use crate::{
    dependencies::{self, DependencyTree, PluginDependencyState, MaybePlugin},
    Registry,
    installer,
    check_for_updates,
    new_registry,
    update_registry,
    Result,
    Error,
};

/// Install dependencies for a project
pub async fn install(config: &Config) -> Result<ResolvedPlugins> {
    let mut resolved: ResolvedPlugins = Default::default();

    if let Some(ref dependencies) = config.dependencies() {

        let registry = new_registry()?;
        let lock_path = LockFile::get_lock_file(config.project());
        let lock = LockFile::load(&lock_path)?;
        let tree =
            dependencies::resolve(config.project(), dependencies, &lock).await?;

        let mut candidates: Vec<(&str, &PluginDependencyState)> = Vec::new();
        find_candidates(config.project(), &registry, &tree, &mut candidates)?;
        if candidates.is_empty() {
            into_resolved(&tree, &mut resolved)?;
        } else {
            let mut lock_installed: LockFile = Default::default();

            let is_current = check_for_updates().await?;
            if !is_current {
                update_registry().await?;
            }

            for (name, state) in candidates {
                debug!("Install {}", name);

                let plugin = match state.maybe_plugin() {
                    MaybePlugin::Plugin(ref plugin) => {
                        plugin.clone() 
                    } 
                    _ => {
                        installer::install(
                            config.project(),
                            &registry,
                            name,
                            state.dependency(), None).await?
                    }
                };

                // Basic verification that the plugin is sane
                check(name, state.dependency(), &plugin)?;

                let lock_plugin = &plugin;
                let lock_entry: LockFileEntry = lock_plugin.try_into()?;
                lock_installed.package.insert(lock_entry);

                info!("Installed {}", plugin);

                resolved.push((state.dependency().clone(), plugin));
            }

            debug!("Writing lock file {}", lock_path.display());
            let result = LockFile::union(lock, lock_installed);
            result.write(&lock_path)?;
        }
    }

    Ok(resolved)
}

fn into_resolved(tree: &DependencyTree, resolved: &mut ResolvedPlugins) -> Result<()> {
    for (name, state) in tree.iter() {
        let dep = state.dependency().clone();

        // This should be safe because we called `state.satisfied()` when finding 
        // installation candidates.
        let plugin = if let MaybePlugin::Plugin(plugin) = state.maybe_plugin() {
            plugin.clone()
        } else {
            return Err(Error::PluginNotSatisfied)
        };

        // Basic verification that the plugin is sane
        check(name, &dep, &plugin)?;

        resolved.push((dep, plugin));

        if !state.transitive().is_empty() {
            into_resolved(state.transitive(), resolved)?;
        }
    }

    Ok(())
}

fn find_candidates<'a, P: AsRef<Path>>(
    project: P,
    registry: &Registry<'_>,
    tree: &'a DependencyTree,
    candidates: &mut Vec<(&'a str, &'a PluginDependencyState)>,
) -> Result<()> {
    for (name, state) in tree.iter() {
        if !state.satisfied()? {
            candidates.push((name, state));
        }
        if !state.transitive().is_empty() {
            find_candidates(project.as_ref(), registry, state.transitive(), candidates)?;
        }
    }
    Ok(())
}

/// Perform some basic checks that a resolved plugin 
/// matches a source dependency.
fn check(name: &str, dep: &Dependency, plugin: &Plugin) -> Result<()> {
    if name != plugin.name() {
        return Err(Error::PluginNameMismatch(
            name.to_string(),
            plugin.name().to_string(),
        ));
    }

    if !dep.version.matches(plugin.version()) {
        return Err(Error::PluginVersionMismatch(
            plugin.name().to_string(),
            plugin.version().to_string(),
            dep.version.to_string(),
        ));
    }

    Ok(())
}
