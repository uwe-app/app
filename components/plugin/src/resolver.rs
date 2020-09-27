use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use log::{info, debug};

use config::{
    dependency::{Dependency, DependencyMap, DependencyTarget},
    lock_file::LockFile,
    lock_file::LockFileEntry,
    plugin::Plugin,
    registry::RegistryItem,
    semver::Version,
    PLUGIN,
};

use crate::{
    installer,
    registry::{self, RegistryAccess},
    Error, Registry, Result,
};

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
type Resolved = Vec<(Dependency, Plugin)>;

/// Resolve the plugins for a collection of dependencies.
pub async fn resolve<P: AsRef<Path>>(
    project: P,
    dependencies: DependencyMap,
) -> Result<DependencyMap> {
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
    resolved: Resolved,
    output: DependencyMap,
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
            output: Default::default(),
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
        let difference = self
            .lock
            .target
            .diff(&self.lock.current)
            .collect::<HashSet<&LockFileEntry>>();

        // Find references that have already been solved
        let mut done: Vec<(Dependency, Plugin)> = 
            self.lock.target.package
            .iter()
            .filter(|entry| {
                let (_dep, solved) = self.intermediate.get(entry).as_ref().unwrap();
                match solved {
                    SolvedReference::Plugin(_) => {
                        return true
                    }
                    _ => {}
                }
                false
            })
            .map(|entry| {
                let (dep, solved) = self.intermediate.get(entry).as_ref().unwrap();
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

            debug!("Installing dependencies");
            self.install_diff(difference).await?;

            debug!("Writing lock file {}", self.lock.path.display());
            // FIXME: restore writing out the new lock file
            //self.lock.target.write(&self.lock.path)?;
        }

        Ok(self)
    }

    /// Install files from the lock file difference.
    async fn install_diff(
        &self,
        difference: HashSet<&LockFileEntry>,
    ) -> Result<()> {
        for entry in difference {
            let (dep, solved) = self.intermediate.get(entry).take().unwrap();
            println!("Install from lock file entry {}", &entry.name);
            match solved {
                SolvedReference::Package(ref _package) => {
                    println!("Installing {:?}", &dep.name);
                    let plugin = installer::install(&self.registry, dep).await?;
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
        //dep.plugin = Some(plugin);
        Ok(())
    }

    /// Get the computed dependency map.
    fn into_inner(self) -> DependencyMap {
        self.output
    }
}

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
async fn solver(
    registry: &Box<dyn RegistryAccess + Send + Sync + 'async_recursion>,
    input: DependencyMap,
    intermediate: &mut IntermediateMap,
    lock: &mut ResolverLock,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
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
            check_plugin(&name, &dep, &plugin, stack)?;
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

        stack.push(name.clone());

        let dependencies: DependencyMap = match solved {
            SolvedReference::Plugin(ref mut plugin) => {
                if let Some(dependencies) = plugin.dependencies.take() {
                    dependencies 
                } else {
                    Default::default()
                }
            }
            // TODO: get dependencies from the package list
            _ => Default::default()
        };

        // If we have nested dependencies recurse 
        if !dependencies.items.is_empty() {
            solver(registry, dependencies, intermediate, lock, stack).await?;
        }

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
    registry: &Registry<'_>,
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

        // Resolve a cached plugin if possible
        if let Some(plugin) = installer::get_cached(registry, dep).await?.take()
        {
            return Ok((version, Some(package), Some(plugin)));
        }

        Ok((version, Some(package), None))
    }
}
