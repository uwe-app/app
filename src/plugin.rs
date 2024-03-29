//! Command functions for upm(1).
use std::fs;
use std::path::PathBuf;

use human_bytes::human_bytes;
use log::{debug, info};
use semver::VersionReq;

use config::{
    href::UrlPath,
    plugin::{
        dependency::{Dependency, DependencyTarget},
        ExactPluginSpec, Plugin, PluginSpec,
    },
};
use plugin::{
    check_for_updates, dependency_installed, get, install_dependency,
    installation_dir, new_registry, peek,
};

use crate::{Error, Result};

/// Install project dependencies.
pub async fn install(project: PathBuf) -> Result<()> {
    let workspace = workspace::open(&project, true, &vec![])?;
    for config in workspace.into_iter() {
        info!("{} ({})", config.host(), config.project().display());
        let resolved = plugin::install(&config).await?;
        info!("Plugins ok ✓ ({})", resolved.len());
    }
    Ok(())
}

/// List project plugin dependencies.
pub async fn list_project(project: PathBuf) -> Result<()> {
    let workspace = workspace::open(&project, true, &vec![])?;
    for config in workspace.into_iter() {
        plugin::list_dependencies(&config).await?;
    }
    Ok(())
}

/// List registry plugins.
pub async fn list_registry(_downloads: bool, _installed: bool) -> Result<()> {
    let registry = new_registry()?;
    let all = registry.all().await?;
    for (name, entry) in all.iter() {
        if let Some((version, _item)) = entry.latest() {
            let installed_versions = registry.installed_versions(entry).await?;
            let is_installed = installed_versions.contains_key(version);
            let mark = if is_installed { "◯" } else { "-" };
            info!("{} {}@{}", mark, name, version);
        }
    }

    info!("Checking for registry updates...");
    let is_current = check_for_updates().await?;
    utils::terminal::clear_previous_line()?;

    if is_current {
        info!("");
        info!("Plugin registry is up to date!");
    } else {
        info!("");
        info!("Plugin registry needs updating, run:");
        info!("");
        info!("upm registry update");
        info!("");
        info!("To refresh the list of available plugnins.");
    }

    Ok(())
}

/// Show plugin information.
pub async fn show(spec: ExactPluginSpec) -> Result<()> {
    let registry = new_registry()?;

    let version_req = if let Some(version) = spec.version() {
        VersionReq::exact(version)
    } else {
        VersionReq::any()
    };

    let (version, _package) =
        registry.resolve(spec.name(), &version_req).await?;
    let fetch_info = get(spec.name(), &version).await?;
    let plugin = peek(&fetch_info.archive).await?;

    info!("{}@{}", plugin.name(), plugin.version());
    info!("");
    info!("{}", plugin.description());
    info!("");

    if let Some(repo) = plugin.repository() {
        info!("Repository: {}", repo);
    }

    // TODO: print author info!

    if !plugin.keywords().is_empty() {
        info!("Keywords: {}", plugin.keywords().join(", "));
    }
    if !plugin.origins().is_empty() {
        let origins = plugin
            .origins()
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>();

        info!("Origins: {}", origins.join(", "));
    }
    if let Some(license) = plugin.license() {
        let licenses = license
            .to_vec()
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>();
        info!("Licenses: {}", licenses.join(", "));
    }

    print_plugin_dependency(&plugin, &None);

    Ok(())
}

fn print_plugin_dependency(plugin: &Plugin, target: &Option<DependencyTarget>) {
    let dependency_message = plugin.to_dependency_toml_string(target);
    let delimiter = utils::terminal::delimiter();
    println!("{}", &delimiter);
    println!("");
    println!("{}", dependency_message);
    println!("");
    println!("{}", &delimiter);
    println!(
        r#"To add this plugin copy the snippet above into the "site.toml" file for the project."#
    );
    println!("{}", &delimiter);
}

/// Add a plugin to the installation folder.
pub async fn add(
    plugin_name: Option<ExactPluginSpec>,
    mut path: Option<PathBuf>,
    mut archive: Option<PathBuf>,
    mut git: Option<String>,
    prefix: Option<UrlPath>,
    force: bool,
) -> Result<()> {
    // Check only a single plugin option is given
    let mut specified = 0usize;
    if plugin_name.is_some() {
        specified += 1
    }
    if path.is_some() {
        specified += 1
    }
    if archive.is_some() {
        specified += 1
    }
    if git.is_some() {
        specified += 1
    }
    if specified == 0 {
        return Err(Error::PluginAddNoTarget);
    } else if specified > 1 {
        return Err(Error::PluginAddMultipleTargets);
    }

    let registry = new_registry()?;
    let project = std::env::current_dir()?;
    info!("Plugins {}", config::plugins_dir()?.display());

    let (name, dep) = if let Some(plugin_spec) = plugin_name {
        let name = plugin_spec.name().to_string();
        let dep: Dependency = plugin_spec.into();
        if !force {
            if let Some(plugin) =
                dependency_installed(&project, &registry, &name, &dep).await?
            {
                return Err(Error::PluginAlreadyInstalled(
                    plugin.name().to_string(),
                    plugin.version().to_string(),
                ));
            }
        };
        (name, dep)
    } else {
        if let Some(path) = path.take() {
            if !path.exists() || !path.is_dir() {
                return Err(Error::NotDirectory(path));
            }
            (
                String::new(),
                DependencyTarget::File {
                    path: path.canonicalize()?,
                }
                .into(),
            )
        } else {
            if let Some(archive) = archive.take() {
                if !archive.exists() || !archive.is_file() {
                    return Err(Error::NotFile(archive));
                }
                (
                    String::new(),
                    DependencyTarget::Archive {
                        archive: archive.canonicalize()?,
                    }
                    .into(),
                )
            } else {
                if let Some(git) = git.take() {
                    (
                        String::new(),
                        DependencyTarget::Repo { git, prefix }.into(),
                    )
                } else {
                    return Err(Error::PluginAddNoTarget);
                }
            }
        }
    };

    match install_dependency(&project, &registry, &name, &dep, force, None)
        .await
    {
        Ok(plugin) => {
            debug!("Location {}", plugin.base().display());
            info!("Installed plugin {}@{} ✓", plugin.name(), plugin.version());

            print_plugin_dependency(&plugin, dep.target());
        }
        Err(e) => {
            if !force {
                if let plugin::Error::PackageOverwrite(name, version, _path) = e
                {
                    return Err(Error::PluginAlreadyInstalled(name, version));
                }
            }
            return Err(Error::from(e));
        }
    }
    Ok(())
}

/// Remove an installed plugin.
pub async fn remove(spec: PluginSpec) -> Result<()> {
    let registry = new_registry()?;
    let results = registry.find(&spec).await?;
    info!("Plugins {}", config::plugins_dir()?.display());

    let mut removed = 0usize;

    if results.is_empty() {
        info!("No installed plugins found matching {}", &spec);
    } else {
        for item in results {
            let dir = installation_dir(item.name(), item.version())?;
            if dir.exists() && dir.is_dir() {
                info!("Remove {}@{}", item.name(), item.version());
                fs::remove_dir_all(&dir)?;
                removed += 1;
            }
        }
    }

    info!("Removed {} version(s) of {} ✓", removed, spec.name());

    Ok(())
}

/// Update the plugin registry cache
pub async fn update_registry() -> Result<()> {
    plugin::update_registry().await?;
    info!("Update complete ✓");
    Ok(())
}

/// Lint a plugin.
pub async fn lint(path: PathBuf, inspect: bool) -> Result<()> {
    let plugin = plugin::lint(path).await?;
    if inspect {
        println!("{}", toml::to_string(&plugin)?);
    }
    info!("Plugin {} ok ✓", &plugin.name);
    Ok(())
}

/// Package a plugin.
pub async fn pack(path: PathBuf) -> Result<()> {
    let target = path.join(config::PACKAGE);
    let source = path;
    let (pkg, digest, plugin) = plugin::pack(&source, &target).await?;
    let size = pkg.metadata()?.len();
    debug!("{}", hex::encode(digest));
    info!("{}", plugin.to_string());
    info!("{} ({})", pkg.display(), human_bytes(size as f64));
    Ok(())
}

/// Publish a plugin.
pub async fn publish(path: PathBuf) -> Result<()> {
    let registry_path = option_env!("UPM_PUBLISH");
    let registry_repo = option_env!("UPM_PUBLISH_REPO");

    if registry_path.is_none() || registry_repo.is_none() {
        log::warn!("Plugin publishing is not available yet.");
        log::warn!("");
        log::warn!("During the alpha and beta plugins are curated, ");
        log::warn!("you may still contribute a plugin by adding ");
        log::warn!("a PR to this repository:");
        log::warn!("");
        log::warn!("https://github.com/uwe-app/plugins");
        log::warn!("");

        return Err(Error::NoPluginPublishPermission);
    }

    plugin::publish(&path).await?;

    Ok(())
}

pub mod clean {
    use std::fs;
    use std::path::PathBuf;
    use std::{thread, time};

    use log::info;
    use pbr::ProgressBar;
    use utils::walk;

    use crate::Result;

    #[derive(Debug)]
    enum CleanTarget {
        File(PathBuf),
        Directory(PathBuf),
    }

    pub async fn all(dry_run: bool) -> Result<()> {
        let mut files = find_archives().await?;
        files.append(&mut find_downloads().await?);
        files.append(&mut find_plugins().await?);
        files.append(&mut find_repositories().await?);
        remove(files, dry_run).await
    }

    pub async fn archives(dry_run: bool) -> Result<()> {
        let files = find_archives().await?;
        remove(files, dry_run).await
    }

    pub async fn downloads(dry_run: bool) -> Result<()> {
        let files = find_downloads().await?;
        remove(files, dry_run).await
    }

    pub async fn plugins(dry_run: bool) -> Result<()> {
        let files = find_plugins().await?;
        remove(files, dry_run).await
    }

    pub async fn repositories(dry_run: bool) -> Result<()> {
        let files = find_repositories().await?;
        remove(files, dry_run).await
    }

    fn format_message(
        base: &PathBuf,
        file: &PathBuf,
        progress: bool,
    ) -> Result<String> {
        let standard = file.strip_prefix(&base)?.to_path_buf();

        let display_path = if progress {
            if let Some(name) = file.file_name() {
                if let Some(parent) = file.parent() {
                    if let Some(parent_name) = parent.file_name() {
                        PathBuf::from(parent_name).join(name)
                    } else {
                        standard
                    }
                } else {
                    standard
                }
            } else {
                standard
            }
        } else {
            standard
        };

        let message = if progress {
            format!("{} ", display_path.to_string_lossy())
        } else {
            display_path.to_string_lossy().into_owned().to_string()
        };
        Ok(message)
    }

    async fn remove(targets: Vec<CleanTarget>, dry_run: bool) -> Result<()> {
        let base = dirs::root_dir()?;

        let mut pb = if !dry_run {
            let mut pb = ProgressBar::new(targets.len() as u64);
            pb.show_speed = false;
            Some(pb)
        } else {
            None
        };

        for target in targets {
            match target {
                CleanTarget::File(path) => {
                    if !dry_run {
                        fs::remove_file(&path)?;
                        if let Some(pb) = pb.as_mut() {
                            let display_path =
                                format_message(&base, &path, true)?;
                            pb.message(&display_path);
                            pb.inc();
                            thread::sleep(time::Duration::from_millis(20));
                        }
                    } else {
                        let display_path = format_message(&base, &path, false)?;
                        info!("{}", &display_path);
                    }
                }
                CleanTarget::Directory(path) => {
                    if !dry_run {
                        fs::remove_dir_all(&path)?;
                        if let Some(pb) = pb.as_mut() {
                            let display_path =
                                format_message(&base, &path, true)?;
                            pb.message(&display_path);
                            pb.inc();
                            thread::sleep(time::Duration::from_millis(20));
                        }
                    } else {
                        let display_path = format_message(&base, &path, false)?;
                        info!("{}", &display_path);
                    }
                }
            }
        }

        if let Some(pb) = pb.as_mut() {
            pb.finish();
            utils::terminal::clear_previous_line()?;
        }

        if !dry_run {
            info!("Clean complete ✓");
        }

        Ok(())
    }

    async fn find_archives() -> Result<Vec<CleanTarget>> {
        find_folders(&dirs::archives_dir()?).await
    }

    async fn find_downloads() -> Result<Vec<CleanTarget>> {
        find_files(&dirs::downloads_dir()?).await
    }

    async fn find_plugins() -> Result<Vec<CleanTarget>> {
        find_folders(&config::plugins_dir()?).await
    }

    async fn find_repositories() -> Result<Vec<CleanTarget>> {
        find_folders(&dirs::repositories_dir()?).await
    }

    async fn find_folders(path: &PathBuf) -> Result<Vec<CleanTarget>> {
        let files = walk::read_dir(path, |f| f.is_dir())?
            .into_iter()
            .map(|f| CleanTarget::Directory(f))
            .collect::<Vec<CleanTarget>>();
        Ok(files)
    }

    async fn find_files(path: &PathBuf) -> Result<Vec<CleanTarget>> {
        let files = walk::read_dir(path, |f| {
            if f.is_file() {
                if let Some(name) = f.file_name() {
                    if name.to_string_lossy().ends_with(".tar.xz") {
                        return true;
                    }
                }
            }
            false
        })?
        .into_iter()
        .map(|f| CleanTarget::File(f))
        .collect::<Vec<CleanTarget>>();
        Ok(files)
    }
}
