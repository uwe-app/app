use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use log::{debug, info, warn};

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
    runtime, Error, Registry, Result,
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
    offline: bool,
) -> Result<ResolvedPlugins> {
    let mut resolver =
        Resolver::new(project.as_ref().to_path_buf(), dependencies, offline)?;
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
    offline: bool,
    updated: bool,
}

impl<'a> Resolver<'a> {
    pub fn new(
        project: PathBuf,
        dependencies: DependencyMap,
        offline: bool,
    ) -> Result<Self> {
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
            offline,
            updated: false,
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
            &mut Default::default(),
            None,
        )
        .await?;
        Ok(self)
    }

    async fn update_registry(&mut self) -> Result<()> {
        if !self.offline {
            if !self.updated {
                info!("Update registry cache");
                runtime::fetch().await?;
                self.updated = true;
            }
        } else {
            warn!("Skip registry update in offline mode");
        }
        Ok(())
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
                let (dep, solved) =
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
            self.update_registry().await?;

            // Refresh the lock file entries in case we can resolve
            // newer versions from the updated registry information
            // FIXME: only run this if the cache registry changed
            let diff = self.refresh(&mut difference).await?;

            // FIXME: support offline flag for installations from the registry
            // FIXME: and from remote repositories

            info!("Installing dependencies");
            self.install_diff(diff).await?;

            info!("Writing lock file {}", self.lock.path.display());
            self.lock.target.write(&self.lock.path)?;
        }

        // Update local scoped plugins with correct attributes
        self.scopes()?;

        // Lock file can be valid and difference is zero
        // but packages are missing because they were deleted
        // from the cache
        self.verify().await?;

        Ok(self)
    }

    async fn verify(&mut self) -> Result<()> {
        for e in self.lock.current.package.iter() {
            if let Some(_) = e.source {
                let dir = installer::installation_dir(&e.name, &e.version)?;
                if !dir.exists() || !dir.is_dir() {
                    return Err(Error::NoPluginInstallDir(dir));
                }
            }
        }
        Ok(())
    }

    fn scopes(&mut self) -> Result<()> {
        let scoped = self
            .resolved
            .iter()
            .enumerate()
            .filter(|(_, (d, _))| d.target.is_some())
            .filter(|(_, (d, _))| {
                if let DependencyTarget::Local { .. } =
                    d.target.as_ref().unwrap()
                {
                    true
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        let scoped = scoped
            .into_iter()
            .map(|(i, (_dep, plugin))| {
                let parent_name = plugin.parent();
                let parent = self
                    .resolved
                    .iter()
                    .cloned()
                    .find(|(_, e)| e.name == parent_name);

                (i, parent)
            })
            .collect::<Vec<_>>();

        for (index, parent) in scoped {
            let (dep, plugin) = self.resolved.get_mut(index).unwrap();

            let (parent_dep, parent_plugin) =
                parent.as_ref().ok_or_else(|| {
                    Error::PluginParentNotFound(
                        plugin.parent(),
                        plugin.name.clone(),
                    )
                })?;

            //println!("Got scoped at {}", index);
            //println!("Got scoped name {}", &plugin.name);
            //println!("Got scoped parent name {:?}", &parent);
            installer::inherit(dep, plugin, parent_plugin, parent_dep)?;
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
                    let plugin = installer::install(
                        &self.project,
                        &self.registry,
                        &dep,
                        None,
                    )
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
    tree: &mut Vec<Dependency>,
    parent: Option<SolvedReference>,
) -> Result<()> {
    if stack.len() > DEPENDENCY_STACK_SIZE {
        return Err(Error::DependencyStackTooLarge(DEPENDENCY_STACK_SIZE));
    }

    for (name, mut dep) in input.into_iter() {
        if stack.contains(&name) {
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
            .unwrap_or(LockFileEntry {
                name: name.to_string(),
                version: version.clone(),
                checksum,
                source: None,
                dependencies: None,
            });

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

        //println!("Solved {:#?}", solved);

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

        let has_features = dep.features.is_some()
            && !dep.features.as_ref().unwrap().is_default();

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
            tree.push(dep.clone());
            solver(
                project,
                registry,
                dependencies,
                intermediate,
                lock,
                stack,
                tree,
                Some(solved.clone()),
            )
            .await?;
            tree.pop();
            stack.pop();
        }

        //println!("Entry is {:#?}", entry);

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
                let plugin =
                    installer::install_path(project, path, None).await?;
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
                    return Err(Error::PluginScopeRequiresParent(
                        dep.to_string(),
                        scope.to_string(),
                    ));
                };

                let plugin =
                    installer::install_local(project, scope, Some(locals))
                        .await?;
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
