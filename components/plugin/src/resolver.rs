use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use log::{debug, info};

use futures::future;

use config::{
    dependency::{Dependency, DependencyMap, DependencyTarget},
    features::FeatureMap,
    lock_file::LockFile,
    lock_file::LockFileEntry,
    plugin::Plugin,
    plugin::ResolvedPlugins,
    registry::RegistryItem,
    semver::Version,
};

use crate::{
    installer,
    reader::read,
    registry::{self, RegistryAccess},
    Error, Registry, Result,
};

static DEPENDENCY_STACK_SIZE: usize = 32;

#[derive(Debug)]
enum SolvedReference {
    /// Local file system packages can be solved to plugins
    /// directly.
    Plugin(Plugin),
    /// Registry references can either be resolved as plugins
    /// if they have already been cached otherwise they need to
    /// be installed which is represented by this type.
    Package(RegistryItem),
}

type IntermediateMap = HashMap<LockFileEntry, (Dependency, SolvedReference)>;

/// Resolve the plugins for a collection of dependencies.
pub async fn resolve<P: AsRef<Path>>(
    project: P,
    dependencies: DependencyMap,
) -> Result<ResolvedPlugins> {
    let mut resolver =
        Resolver::new(project.as_ref().to_path_buf(), dependencies)?;
    resolver.solve().await?;
    resolver.install().await?;
    resolver.prepare()?;
    Ok(resolver.into_inner())
}

/// Stores lock file information.
struct ResolverLock {
    path: PathBuf,
    current: LockFile,
    target: LockFile,
}

impl ResolverLock {
    fn new(path: PathBuf) -> Result<Self> {
        let current = LockFile::load(&path)?;
        Ok(ResolverLock {
            path,
            current,
            target: Default::default(),
        })
    }
}

/// Manges the information required to solve all dependencies.
struct Resolver<'a> {
    dependencies: DependencyMap,
    registry: Registry<'a>,
    lock: ResolverLock,
    intermediate: IntermediateMap,
    resolved: ResolvedPlugins,
}

impl<'a> Resolver<'a> {
    pub fn new(project: PathBuf, dependencies: DependencyMap) -> Result<Self> {
        let registry = registry::new_registry()?;
        let path = LockFile::get_lock_file(&project);
        let lock = ResolverLock::new(path)?;
        Ok(Self {
            dependencies,
            registry,
            lock,
            intermediate: HashMap::new(),
            resolved: Vec::new(),
        })
    }

    /// Solve the dependency tree using the current and
    /// target lock files.
    async fn solve(&mut self) -> Result<&mut Resolver<'a>> {
        solver(
            &self.registry,
            std::mem::take(&mut self.dependencies),
            &mut self.intermediate,
            &mut self.lock,
            &mut Default::default(),
        )
        .await?;
        Ok(self)
    }

    /// Calculate the lock file difference and install plugins when
    /// the difference is not empty.
    async fn install(&mut self) -> Result<&mut Resolver<'a>> {
        let mut difference = self
            .lock
            .target
            .diff(&self.lock.current)
            .map(|l| l.clone())
            .collect::<HashSet<LockFileEntry>>();

        // Find references that have already been solved
        let mut done: Vec<(Dependency, Plugin)> = self
            .lock
            .target
            .package
            .iter()
            .filter(|entry| {
                let (_dep, solved) =
                    self.intermediate.get(entry).as_ref().unwrap();
                match solved {
                    SolvedReference::Plugin(_) => return true,
                    _ => {}
                }
                false
            })
            .map(|entry| {
                let (dep, solved) =
                    self.intermediate.get(entry).as_ref().unwrap();
                match solved {
                    SolvedReference::Plugin(ref plugin) => {
                        return Some((dep.clone(), plugin.clone()))
                    }
                    _ => {}
                }
                None
            })
            .map(|o| o.unwrap())
            .collect();

        // Move the resolved references
        self.resolved.append(&mut done);

        if !difference.is_empty() {
            info!("Update registry cache");
            let prefs = preference::load()?;
            cache::update(&prefs, vec![cache::CacheComponent::Runtime])?;

            // Refresh the lock file entries in case we can resolve 
            // newer versions from the updated registry information
            // FIXME: only run this if the cache registry changed
            let diff = self.refresh(&mut difference).await?;

            debug!("Installing dependencies");
            self.install_diff(diff).await?;

            debug!("Writing lock file {}", self.lock.path.display());
            // FIXME: restore writing out the new lock file
            //self.lock.target.write(&self.lock.path)?;
        }

        Ok(self)
    }

    async fn refresh(&mut self, difference: &mut HashSet<LockFileEntry>) -> Result<HashSet<LockFileEntry>> {
        let mut refreshed = self.refresh_lock(difference).await?;

        // We need to update the intermediate map to reflect the change
        // to the lock file entry
        for (entry, refresh) in refreshed.iter_mut() {
            if let Some(ref replacement) = refresh {
                let (dep, solved) = 
                    self.intermediate.remove(entry).take().unwrap();
                self.intermediate.insert(replacement.clone(), (dep, solved));
            } 
        }

        Ok(refreshed
            .drain(..)
            .map(|(e, r)| r.unwrap_or(e.clone()))
            .collect::<HashSet<_>>())
    }

    /// Update lock file entries after the registry has been 
    /// updated in case a newer version can be located in the 
    /// fresh registry.
    async fn refresh_lock<'b>(&self, diff: &'b mut HashSet<LockFileEntry>)
        -> Result<Vec<(&'b LockFileEntry, Option<LockFileEntry>)>> {
        let items = diff
            .iter()
            .map(|e| self.refresh_lock_entry(e))
            .collect::<Vec<_>>();
        Ok(future::try_join_all(items).await?)
    }

    /// Refresh a single lock file entry, if a newer version can
    /// be resolved in the registry we also return a copy with 
    /// the newer version.
    async fn refresh_lock_entry<'b>(&self, e: &'b LockFileEntry)
        -> Result<(&'b LockFileEntry, Option<LockFileEntry>)> {
        // Need the source dependency for the version request
        let (dep, _solved) =
            self.intermediate.get(&e).as_ref().unwrap();

        // Try to resolve the package again
        let (version, _package) = installer::resolve_package(
            &self.registry, &e.name, &dep.version).await?;

        let mut output: Option<LockFileEntry> = None;
        if version > e.version {
            let mut copy = e.clone();
            copy.version = version;
            output = Some(copy);
        }
        Ok((e, output))
    }

    /// Install files from the lock file difference.
    async fn install_diff(
        &mut self,
        difference: HashSet<LockFileEntry>,
    ) -> Result<()> {
        for entry in difference {
            let (dep, solved) =
                self.intermediate.remove(&entry).take().unwrap();
            match solved {
                SolvedReference::Package(ref _package) => {
                    let plugin =
                        installer::install(&self.registry, &dep).await?;
                    self.resolved.push((dep, plugin));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn prepare(&mut self) -> Result<()> {
        for (dep, _) in self.resolved.iter_mut() {
            dep.prepare()?;
        }
        Ok(())
    }

    /// Get the computed dependency map.
    fn into_inner(self) -> ResolvedPlugins {
        self.resolved
    }
}

#[async_recursion]
async fn solver(
    registry: &Box<dyn RegistryAccess + Send + Sync + 'async_recursion>,
    input: DependencyMap,
    intermediate: &mut IntermediateMap,
    lock: &mut ResolverLock,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
        if stack.len() > DEPENDENCY_STACK_SIZE {
            return Err(Error::DependencyStackTooLarge(DEPENDENCY_STACK_SIZE));
        } else if stack.contains(&name) {
            return Err(Error::CyclicDependency(name));
        }

        dep.name = Some(name.clone());

        let mut entry = lock
            .current
            .package
            .iter()
            .find_map(|e| {
                if e.name == name {
                    return Some(e);
                }
                None
            })
            .map(|e| e.clone())
            .unwrap_or(Default::default());

        let (version, mut package, mut plugin) =
            resolve_version(registry, &dep).await?;

        let checksum = if let Some(ref pkg) = package {
            Some(pkg.digest.clone())
        } else {
            None
        };

        let mut solved = if let Some(plugin) = plugin.take() {
            check_plugin(&name, &dep, &plugin)?;
            SolvedReference::Plugin(plugin)
        } else if let Some(package) = package.take() {
            SolvedReference::Package(package)
        } else {
            return Err(Error::DependencyNotFound(dep.to_string()));
        };

        if entry == LockFileEntry::default() {
            entry = LockFileEntry {
                name: name.to_string(),
                version: version.clone(),
                checksum,
                source: None,
                dependencies: None,
            }
        }

        // FIXME: filter out based on dependency features

        let dependencies: DependencyMap = match solved {
            SolvedReference::Plugin(ref mut plugin) => {
                if let Some(dependencies) = plugin.dependencies.take() {
                    dependencies
                } else {
                    Default::default()
                }
            }
            SolvedReference::Package(ref mut package) => {
                package.to_dependency_map()
            }
        };

        // If we have nested dependencies recurse
        if !dependencies.is_empty() {
            //println!("Entering {:#?}", dep.name);
            //println!("Solved {:#?}", solved);

            let feature_map: &Option<FeatureMap> = match solved {
                SolvedReference::Plugin(ref plugin) => &plugin.features,
                SolvedReference::Package(ref package) => &package.features,
            };

            let dependencies = dependencies.filter(&dep, feature_map)?;

            stack.push(name.clone());
            solver(registry, dependencies, intermediate, lock, stack).await?;
            stack.pop();
        }

        stack.pop();

        println!("Entry is {:#?}", entry);

        // Got a dependency that is already resolved so we need to ensure
        // if fills the same requirements as the previous plugin match
        if intermediate.contains_key(&entry) {
            let (dep_first, _) = intermediate.get(&entry).as_ref().unwrap();
            if !dep_first.version.matches(&version) {
                return Err(Error::IncompatibleDependency(
                    dep.to_string(),
                    dep_first.to_string(),
                ));
            }
        }

        let tmp = entry.clone();

        // Store the lock file entry so we can diff later
        // to determine which dependencies need installing
        lock.target.package.insert(entry);

        // Store the intermediate entries.
        intermediate.entry(tmp).or_insert((dep, solved));
    }

    Ok(())
}

fn check_plugin(name: &str, dep: &Dependency, plugin: &Plugin) -> Result<()> {
    if name != plugin.name {
        return Err(Error::PluginNameMismatch(
            name.to_string(),
            plugin.name.clone(),
        ));
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
    registry: &Registry<'_>,
    dep: &Dependency,
) -> Result<(Version, Option<RegistryItem>, Option<Plugin>)> {
    if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => {
                let plugin = read(path).await?;
                Ok((plugin.version.clone(), None, Some(plugin)))
            }
            DependencyTarget::Archive { .. } => todo!(),
        }
    } else {
        // Get version from registry
        let name = dep.name.as_ref().unwrap();
        let (version, package) =
            installer::resolve_package(registry, name, &dep.version).await?;

        // Resolve a cached plugin if possible
        if let Some(plugin) = installer::get_cached(registry, dep).await?.take()
        {
            return Ok((version, Some(package), Some(plugin)));
        }

        Ok((version, Some(package), None))
    }
}
