use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use log::debug;

use config::{
    dependency::{Dependency, DependencyMap, DependencyTarget},
    features::FeatureMap,
    lock_file::LockFile,
    lock_file::LockFileEntry,
    plugin::Plugin,
    registry::RegistryItem,
    semver::Version,
};

use crate::{
    installer,
    registry::{self, RegistryAccess},
    Error, Registry, Result,
};

static DEPENDENCY_STACK_SIZE: usize = 32;

// Top-level of a project dependencies
pub type DependencyTree = BTreeMap<String, PluginDependencyState>;

/// Resolve the dependencies for a project.
pub async fn resolve<P: AsRef<Path>>(
    project: P,
    dependencies: &DependencyMap,
    lock: &LockFile,
) -> Result<DependencyTree> {
    let mut out = BTreeMap::new();
    let mut solver =
        Solver::new(project.as_ref().to_path_buf(), dependencies, lock)?;
    solver.solve(&mut out).await?;
    Ok(out)
}

#[derive(Debug, Clone)]
pub enum MaybePlugin {
    /// Plugin could not be found
    NotFound,

    /// Local file system and archive targets and lock file matches
    /// that have a corresponding plugin installed can
    /// be resolved to a plugin during this pass.
    Plugin(Plugin),

    /// If a plugin cannot be resolved immediately because it is not
    /// yet installed we can use the registry item to find available
    /// features so that we can filter the dependency tree on features
    /// assigned to the dependency.
    ///
    /// By keeping a reference to the registry item and it's features
    /// we know which features are valid for the target dependency.
    Package(RegistryItem),
}

#[derive(Debug)]
pub struct PluginDependencyState {
    /// The name of the dependency / plugin.
    name: String,

    /// The source dependency.
    dependency: Dependency,

    /// Maybe a resolved plugin.
    plugin: MaybePlugin,

    /// A resolved version when available, if a lock
    /// file entry is available this will be the
    /// version from the lock file otherwise it is a
    /// candidate version resolved from the registry.
    version: Option<Version>,

    /// Registry package if it exists.
    package: Option<RegistryItem>,

    /// An existing lock file entry.
    entry: Option<LockFileEntry>,

    /// Transitive dependencies.
    transitive: DependencyTree,
}

impl PluginDependencyState {
    fn new(
        name: String,
        dependency: Dependency,
        plugin: MaybePlugin,
        version: Option<Version>,
        package: Option<RegistryItem>,
        entry: Option<LockFileEntry>,
    ) -> Self {
        Self {
            name,
            dependency,
            plugin,
            version,
            package,
            entry,
            transitive: Default::default(),
        }
    }

    pub fn target_version(&self) -> &Option<Version> {
        &self.version
    }

    pub fn is_local_scope(&self) -> bool {
        if let Some(ref target) = self.dependency.target {
            if let DependencyTarget::Local { .. } = target {
                return true;
            }
        }
        false
    }

    pub fn maybe_plugin(&self) -> &MaybePlugin {
        &self.plugin
    }

    pub fn dependency(&self) -> &Dependency {
        &self.dependency
    }

    pub fn transitive(&self) -> &DependencyTree {
        &self.transitive
    }

    pub fn not_found(&self) -> bool {
        if let MaybePlugin::NotFound = self.plugin {
            return true;
        }
        false
    }

    /// Is the dependency completely satisfied?
    pub fn satisfied(&self) -> Result<bool> {
        let has_lock_file_entry = self.entry.is_some();
        let has_plugin = if let MaybePlugin::Plugin(_) = self.plugin {
            true
        } else {
            false
        };

        let mut is_installed = false;
        let mut satisfies_range = false;

        if let Some(ref version) = self.version {
            let range = self.dependency().range();
            satisfies_range = range.matches(version);
            if self.is_local_scope() {
                is_installed = true;
            } else {
                if installer::is_installed(&self.name, version)? {
                    is_installed = true;
                }
            }
        }

        println!("has_lockfile_entry {}", has_lock_file_entry);
        println!("has_plugin {}", has_plugin);
        println!("is_installed {}", is_installed);
        println!("satisfies_range {}", satisfies_range);

        Ok(
            has_lock_file_entry
                && has_plugin
                && is_installed
                && satisfies_range,
        )
    }
}

/// Solve the dependencies for a project by building a tree
/// representing direct and transitive dependencies and
/// comparing to a lock file (optionally available) and
/// the plugins that are already installed.
struct Solver<'a> {
    project: PathBuf,
    dependencies: &'a DependencyMap,
    registry: Registry<'a>,
    lock: &'a LockFile,
}

impl<'a> Solver<'a> {
    pub fn new(
        project: PathBuf,
        dependencies: &'a DependencyMap,
        lock: &'a LockFile,
    ) -> Result<Self> {
        let registry = registry::new_registry()?;
        Ok(Self {
            project,
            dependencies,
            registry,
            lock,
        })
    }

    /// Solve the dependency tree using the current lock file.
    async fn solve(&mut self, out: &mut DependencyTree) -> Result<()> {
        solver(
            &self.project,
            &self.registry,
            &mut self.dependencies,
            &mut self.lock,
            &mut Default::default(),
            out,
            None,
        )
        .await?;
        Ok(())
    }
}

#[async_recursion]
async fn solver(
    project: &PathBuf,
    registry: &Box<dyn RegistryAccess + Send + Sync + 'async_recursion>,
    input: &DependencyMap,
    lock: &LockFile,
    stack: &mut Vec<String>,
    tree: &mut DependencyTree,
    parent: Option<MaybePlugin>,
) -> Result<()> {
    if stack.len() > DEPENDENCY_STACK_SIZE {
        return Err(Error::DependencyStackTooLarge(DEPENDENCY_STACK_SIZE));
    }

    for (name, dep) in input.iter() {
        debug!("Solving {}", name);

        if stack.contains(&name) {
            return Err(Error::CyclicDependency(name.to_string()));
        }

        let entry = lock
            .package
            .iter()
            .find_map(|e| {
                if &e.name == name {
                    return Some(e);
                }
                None
            })
            .map(|e| e.clone());

        let (version, mut package, mut plugin) =
            resolve_version(project, registry, name, &dep, &parent).await?;

        let target_version: Option<Version> = if let Some(ref entry) = entry {
            Some(entry.version().clone())
        } else {
            version.clone()
        };

        let solved = if let Some(plugin) = plugin.take() {
            MaybePlugin::Plugin(plugin)
        } else if let Some(package) = package.take() {
            MaybePlugin::Package(package)
        } else {
            MaybePlugin::NotFound
        };

        let default_dependencies: DependencyMap = Default::default();
        let dependencies: &DependencyMap = match solved {
            MaybePlugin::Plugin(ref plugin) => plugin.dependencies(),
            MaybePlugin::Package(ref package) => package.dependencies(),
            MaybePlugin::NotFound => &default_dependencies,
        };

        let has_features = dep.features.is_some()
            && !dep.features.as_ref().unwrap().is_default();

        let state = PluginDependencyState::new(
            name.to_string(),
            dep.clone(),
            solved.clone(),
            target_version,
            package,
            entry,
        );
        tree.insert(name.clone(), state);

        // If we have nested dependencies recurse
        if !dependencies.is_empty() || has_features {
            let default_features: FeatureMap = Default::default();
            let feature_map: &FeatureMap = match solved {
                MaybePlugin::Plugin(ref plugin) => plugin.features(),
                MaybePlugin::Package(ref package) => package.features(),
                MaybePlugin::NotFound => &default_features,
            };

            // Filter nested dependencies to resolve depending upon the
            // requested and declared features.
            let dependencies = dependencies.filter(&dep, feature_map)?;

            stack.push(name.clone());

            let mut transitive: DependencyTree = Default::default();

            solver(
                project,
                registry,
                &dependencies,
                lock,
                stack,
                &mut transitive,
                Some(solved),
            )
            .await?;

            let last_state = tree.get_mut(name).unwrap();
            last_state.transitive = transitive;

            stack.pop();
        }
    }

    Ok(())
}

async fn resolve_version<P: AsRef<Path>>(
    project: P,
    registry: &Registry<'_>,
    name: &str,
    dep: &Dependency,
    parent: &Option<MaybePlugin>,
) -> Result<(Option<Version>, Option<RegistryItem>, Option<Plugin>)> {
    debug!("Resolving version {}", name);
    debug!("Resolving target {:?}", dep.target);

    if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => {
                debug!("Resolve local folder path {:?}", path);
                let plugin =
                    installer::install_path(project, path, None).await?;
                Ok((Some(plugin.version().clone()), None, Some(plugin)))
            }
            DependencyTarget::Archive { ref archive } => {
                debug!("Resolve local archive {:?}", archive);
                let plugin = installer::peek(archive).await?;
                Ok((Some(plugin.version().clone()), None, Some(plugin)))
            }

            // NOTE: This potentially requires a network connection!
            DependencyTarget::Repo {
                ref git,
                ref prefix,
            } => {
                debug!("Resolve local repository {:?}", git);
                let plugin =
                    installer::install_repo(project, git, prefix, true).await?;
                Ok((Some(plugin.version().clone()), None, Some(plugin)))
            }

            DependencyTarget::Local { ref scope } => {
                debug!("Resolve local plugin scope {:?}", scope);
                let locals = if let Some(ref parent) = parent {
                    match parent {
                        MaybePlugin::Plugin(ref plugin) => {
                            plugin.plugins().clone()
                        }
                        MaybePlugin::Package(ref package) => {
                            package.plugins().clone()
                        }
                        _ => HashMap::new(),
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
                Ok((Some(plugin.version().clone()), None, Some(plugin)))
            }
        }
    } else {
        // Get version from registry
        match registry.resolve(name, &dep.version).await {
            Ok((version, package)) => {
                debug!("Resolved registry package for {:?}", &version);

                // Resolve a cached plugin if possible
                if let Some(plugin) = installer::version_installed(
                    project,
                    registry,
                    name,
                    &version,
                    Some(&package),
                )
                .await?
                .take()
                {
                    return Ok((Some(version), Some(package), Some(plugin)));
                }

                Ok((Some(version), Some(package), None))
            }

            Err(_) => Ok((None, None, None)),
        }
    }
}
