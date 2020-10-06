use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use log::info;

use futures::future;

use config::{
    dependency::{Dependency, DependencyMap, DependencyTarget},
    features::FeatureMap,
    lock_file::LockFile,
    lock_file::LockFileEntry,
    plugin::{Plugin, PluginSource, ResolvedPlugins},
    registry::RegistryItem,
    semver::Version,
};

use crate::{
    installer,
    registry::{self, RegistryAccess},
    Error, Registry, Result,
};

static DEPENDENCY_STACK_SIZE: usize = 32;

#[derive(Debug, Clone)]
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
    project: PathBuf,
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
            project,
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
            &self.project,
            &self.registry,
            std::mem::take(&mut self.dependencies),
            &mut self.intermediate,
            &mut self.lock,
            &mut Default::default(),
            None,
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
            //info!("Update registry cache");
            //cache::update(vec![cache::CacheComponent::Runtime])?;

            // Refresh the lock file entries in case we can resolve
            // newer versions from the updated registry information
            // FIXME: only run this if the cache registry changed
            let diff = self.refresh(&mut difference).await?;

            info!("Installing dependencies");
            self.install_diff(diff).await?;

            info!("Writing lock file {}", self.lock.path.display());
            // FIXME: restore writing out the new lock file
            //self.lock.target.write(&self.lock.path)?;

            // Update local scoped plugins with correct attributes
            self.scopes()?;
        }

        Ok(self)
    }

    fn scopes(&mut self) -> Result<()> {
        let scoped = self.resolved
            .iter()
            .enumerate()
            .filter(|(i, (d, _))| d.target.is_some())
            .filter(|(i, (d, _))| {
                if let DependencyTarget::Local { ref scope } = d.target.as_ref().unwrap() {
                    true
                } else { false }
            })
            .collect::<Vec<_>>();

        let scoped = scoped
            .into_iter()
            .map(|(i, (dep, plugin))| {
                let parent_name = plugin.parent();
                let parent = self.resolved
                    .iter()
                    .cloned()
                    .map(|(_, e)| e)
                    .find(|e| e.name == parent_name);

                (i, (plugin.clone(), parent))
            })
            .collect::<Vec<_>>();

        for (index, (plugin, parent)) in scoped {
            let parent = parent.as_ref().ok_or_else(
                || Error::PluginParentNotFound(plugin.parent(), plugin.name))?;

            //println!("Got scoped at {}", index);
            //println!("Got scoped name {}", &plugin.name);
            //println!("Got scoped parent name {:?}", &parent);
            let (_, plugin) = self.resolved.get_mut(index).unwrap();
            installer::inherit(plugin, parent)?;
        }

        Ok(())
    }

    async fn refresh(
        &mut self,
        difference: &mut HashSet<LockFileEntry>,
    ) -> Result<HashSet<LockFileEntry>> {
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
    async fn refresh_lock<'b>(
        &self,
        diff: &'b mut HashSet<LockFileEntry>,
    ) -> Result<Vec<(&'b LockFileEntry, Option<LockFileEntry>)>> {
        let items = diff
            .iter()
            .map(|e| self.refresh_lock_entry(e))
            .collect::<Vec<_>>();
        Ok(future::try_join_all(items).await?)
    }

    /// Refresh a single lock file entry, if a newer version can
    /// be resolved in the registry we also return a copy with
    /// the newer version.
    async fn refresh_lock_entry<'b>(
        &self,
        e: &'b LockFileEntry,
    ) -> Result<(&'b LockFileEntry, Option<LockFileEntry>)> {
        // Need the source dependency for the version request
        let (dep, _solved) = self.intermediate.get(&e).as_ref().unwrap();

        let mut output: Option<LockFileEntry> = None;

        // Ensure we only refresh for registry dependencies
        // otherwise this can error for `path` references
        if dep.target.is_none() {
            // Try to resolve the package again
            let (version, _package) = installer::resolve_package(
                &self.registry,
                &e.name,
                &dep.version,
            )
            .await?;
            if version > e.version {
                let mut copy = e.clone();
                copy.version = version;
                output = Some(copy);
            }
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
                        installer::install(&self.project, &self.registry, &dep, None)
                            .await?;
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
    project: &PathBuf,
    registry: &Box<dyn RegistryAccess + Send + Sync + 'async_recursion>,
    input: DependencyMap,
    intermediate: &mut IntermediateMap,
    lock: &mut ResolverLock,
    stack: &mut Vec<String>,
    parent: Option<SolvedReference>,
) -> Result<()> {

    for (name, mut dep) in input.into_iter() {
        if stack.len() > DEPENDENCY_STACK_SIZE {
            return Err(Error::DependencyStackTooLarge(DEPENDENCY_STACK_SIZE));
        } else if stack.contains(&name) {
            return Err(Error::CyclicDependency(name));
        }

        dep.name = Some(name.clone());

        let (version, mut package, mut plugin) =
            resolve_version(project, registry, &dep, &parent).await?;

        let checksum = if let Some(ref pkg) = package {
            Some(pkg.digest.clone())
        } else {
            None
        };

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
            .unwrap_or(
                LockFileEntry {
                    name: name.to_string(),
                    version: version.clone(),
                    checksum,
                    source: None,
                    dependencies: None,
                }
            );

        let mut solved = if let Some(plugin) = plugin.take() {
            // TODO: ensure this is set for SolvedReference::Package
            if let Some(ref source) = plugin.source() {
                if let PluginSource::Registry(ref url) = source {
                    entry.source.get_or_insert(url.clone());
                }
            }
            if let Some(ref checksum) = plugin.checksum() {
                entry.checksum.get_or_insert(checksum.clone());
            }
            SolvedReference::Plugin(plugin)
        } else if let Some(package) = package.take() {
            SolvedReference::Package(package)
        } else {
            return Err(Error::DependencyNotFound(dep.to_string()));
        };

        check(&name, &dep, &solved)?;

        let dependencies: DependencyMap = match solved {
            SolvedReference::Plugin(ref mut plugin) => {
                if !plugin.dependencies().is_empty() {
                    plugin.dependencies().clone()
                } else {
                    Default::default()
                }
            }
            SolvedReference::Package(ref mut package) => {
                package.dependencies().clone()
            }
        };

        let has_features = dep.features.is_some() && !dep.features.as_ref().unwrap().is_default();

        // If we have nested dependencies recurse
        if !dependencies.is_empty() || has_features {
            let feature_map: &FeatureMap = match solved {
                SolvedReference::Plugin(ref plugin) => plugin.features(),
                SolvedReference::Package(ref package) => package.features(),
            };

            // Filter nested dependencies to resolve depending upon the
            // requested and declared features.
            let dependencies = dependencies.filter(&dep, feature_map)?;

            stack.push(name.clone());
            solver(
                project,
                registry,
                dependencies,
                intermediate,
                lock,
                stack,
                Some(solved.clone()),
            ).await?;
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

async fn resolve_version<P: AsRef<Path>>(
    project: P,
    registry: &Registry<'_>,
    dep: &Dependency,
    parent: &Option<SolvedReference>,
) -> Result<(Version, Option<RegistryItem>, Option<Plugin>)> {

    if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => {
                let plugin = installer::install_path(project, path, None).await?;
                Ok((plugin.version.clone(), None, Some(plugin)))
            }
            DependencyTarget::Archive { ref archive } => {
                let plugin =
                    installer::install_archive(project, archive).await?;
                Ok((plugin.version.clone(), None, Some(plugin)))
            }
            DependencyTarget::Repo { ref git } => {
                let plugin = installer::install_repo(project, git).await?;
                Ok((plugin.version.clone(), None, Some(plugin)))
            }
            DependencyTarget::Local { ref scope } => {

                let locals = if let Some(ref parent) = parent {
                    match parent {
                        SolvedReference::Plugin(ref plugin) => {
                            plugin.plugins().clone()
                        }
                        SolvedReference::Package(ref package) => {
                           package.plugins().clone()
                        }
                    }
                } else {
                    return Err(
                        Error::PluginScopeRequiresParent(
                            dep.to_string(), scope.to_string()))
                };

                let plugin = installer::install_local(project, scope, Some(locals)).await?;
                Ok((plugin.version.clone(), None, Some(plugin)))
            }
        }
    } else {
        // Get version from registry
        let name = dep.name.as_ref().unwrap();
        let (version, package) =
            installer::resolve_package(registry, name, &dep.version).await?;

        // Resolve a cached plugin if possible
        if let Some(plugin) =
            installer::get_cached(project, registry, dep).await?.take()
        {
            return Ok((version, Some(package), Some(plugin)));
        }

        Ok((version, Some(package), None))
    }
}

fn check(name: &str, dep: &Dependency, solved: &SolvedReference) -> Result<()> {
    let (s_name, s_version) = match solved {
        SolvedReference::Plugin(ref plugin) => (&plugin.name, &plugin.version),
        SolvedReference::Package(ref package) => {
            (&package.name, &package.version)
        }
    };

    if name != s_name {
        return Err(Error::PluginNameMismatch(
            name.to_string(),
            s_name.clone(),
        ));
    }

    if !dep.version.matches(s_version) {
        return Err(Error::PluginVersionMismatch(
            s_name.clone(),
            s_version.to_string(),
            dep.version.to_string(),
        ));
    }

    Ok(())
}
