//! Command functions for upm(1).
use std::fs;
use std::path::PathBuf;

use human_bytes::human_bytes;
use log::{debug, info, warn};
use url::Url;
use semver::VersionReq;

use config::plugin::{dependency::Dependency, ExactPluginSpec, PluginSpec};
use plugin::{
    get, show,
    dependency_installed, install_archive, install_folder, install_registry,
    install_repo, installation_dir, new_registry,
};

use crate::{Error, Result};

#[derive(Debug)]
pub enum InstallSpec {
    Folder(PathBuf),
    Archive(PathBuf),
    Repo(Url),
    Plugin(ExactPluginSpec),
}

/// List plugins.
pub async fn list(
    _downloads: bool,
    _installed: bool,
) -> Result<()> {
    let registry = new_registry()?;
    let all = registry.all().await?;
    for (name, entry) in all.iter() {
        if let Some((version, item)) = entry.latest() {
            let installed_versions = registry.installed_versions(entry).await?;
            let is_installed = installed_versions.contains(item);
            let mark = if is_installed { "◯" } else { "-" };
            info!("{} {}@{}", mark, name, version);
            //info!(r#""{}" = "{}""#, name, version);
        }
    }
    Ok(())
}

/// Show plugin information.
pub async fn info(spec: ExactPluginSpec) -> Result<()> {
    let registry = new_registry()?;

    let version_req = if let Some(version) = spec.version() {
        VersionReq::exact(version)
    } else { VersionReq::any() };

    let (version, _package) = registry.resolve(spec.name(), &version_req).await?;
    let fetch_info = get(spec.name(), &version).await?;
    let plugin = show(&fetch_info.archive).await?;

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

        let origins = plugin.origins()
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

    info!("");
    info!(r#""{}" = "^{}""#, plugin.name(), plugin.version());

    Ok(())
}

/// Install a plugin.
pub async fn install(spec: InstallSpec, force: bool) -> Result<()> {
    let registry = new_registry()?;
    let project = std::env::current_dir()?;
    info!("Plugins {}", config::plugins_dir()?.display());

    let result = match spec {
        InstallSpec::Plugin(plugin_spec) => {
            let dep: Dependency = plugin_spec.into();
            if !force {
                if let Some(plugin) =
                    dependency_installed(&project, &registry, &dep).await?
                {
                    return Err(Error::PluginAlreadyInstalled(
                        plugin.name().to_string(),
                        plugin.version().to_string(),
                    ));
                }
            };

            install_registry(&project, &registry, &dep).await
        }
        InstallSpec::Folder(path) => {
            install_folder(&project, &path, force).await
        }
        InstallSpec::Archive(path) => {
            install_archive(&project, &path, force).await
        }
        InstallSpec::Repo(url) => {
            install_repo(&project, &url, force).await
        }
    };

    match result {
        Ok(plugin) => {
            debug!("Location {}", plugin.base().display());
            info!(
                "Installed plugin {}@{} ✓",
                plugin.name(),
                plugin.version()
            );
        }
        Err(e) => {
            if !force {
                if let plugin::Error::PackageOverwrite(
                    name,
                    version,
                    _path,
                ) = e
                {
                    return Err(Error::PluginAlreadyInstalled(
                        name, version,
                    ));
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

    if results.is_empty() {
        info!("No installed plugins found matching {}", &spec);
    } else {
        for item in results {
            let dir = installation_dir(item.name(), item.version())?;
            if dir.exists() && dir.is_dir() {
                info!("Remove {}@{}", item.name(), item.version());
                fs::remove_dir_all(&dir)?;
                info!("Deleted {} ✓", dir.display());
            } else {
                warn!(
                    "Plugin {}@{} is not installed!",
                    item.name(),
                    item.version()
                );
            }
        }
    }

    Ok(())
}

/// Update the plugin registry cache
pub async fn update() -> Result<()> {
    scm::system_repo::fetch_registry().await?;
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

/// Remove all cached plugins.
pub async fn clean() -> Result<()> {
    let target = config::plugins_dir()?;
    if target.exists() && target.is_dir() {
        info!("Remove {}", target.display());
        fs::remove_dir_all(&target)?;
    }
    Ok(())
}
