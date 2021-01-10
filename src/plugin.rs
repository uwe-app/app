use std::fs;
use std::path::PathBuf;

use human_bytes::human_bytes;
use log::{debug, info, warn};
use url::Url;

use config::plugin::{ExactPluginSpec, PluginSpec, dependency::Dependency};
use plugin::{
    new_registry,
    installed,
    install_registry,
    install_path,
    install_repo,
    install_archive,
    installation_dir,
};

use crate::{Error, Result};

#[derive(Debug)]
pub enum InstallSpec {
    Folder(PathBuf),
    Archive(PathBuf),
    Repo(Url),
    Plugin(ExactPluginSpec),
}

/// Install a plugin.
pub async fn install(spec: InstallSpec) -> Result<()> {
    let registry = new_registry()?;
    let project = std::env::current_dir()?;
    info!("Plugins {}", config::plugins_dir()?.display());

    // TODO: support --force overwriting?

    match spec {
        InstallSpec::Plugin(plugin_spec) => {
            let dep: Dependency = plugin_spec.into();
            let (plugin, cached) = if let Some(plugin) = installed(&project, &registry, &dep).await? {
                (plugin, true) 
            } else {
                (install_registry(&project, &registry, &dep).await?, false)
            };

            if cached {
                info!("Plugin {}@{} is already installed ✓", plugin.name(), plugin.version());
            } else {
                debug!("{}", plugin.base().display());
                info!("Installed plugin {}@{} ✓", plugin.name(), plugin.version());
            }
        },
        InstallSpec::Folder(path) => {
            let plugin = install_path(&project, &path, None).await?;
            // TODO: copy files to the install location!

            info!("{}", plugin.base().display());
            info!("Installed plugin {}@{} ✓", plugin.name(), plugin.version());
        },
        InstallSpec::Archive(path) => {
            // TODO: install to a standard location!

            let plugin = install_archive(&project, &path).await?;
            debug!("{}", plugin.base().display());
            info!("Installed plugin {}@{} ✓", plugin.name(), plugin.version());
        },
        InstallSpec::Repo(url) => {
            println!("Install from repository {:?}", &url);
            let plugin = install_repo(&project, &url).await?;
            debug!("{}", plugin.base().display());
            info!("Installed plugin {}@{} ✓", plugin.name(), plugin.version());
        },
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
                warn!("Plugin {}@{} is not installed!", item.name(), item.version());
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
