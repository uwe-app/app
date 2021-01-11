use std::fs;
use std::path::{Path, PathBuf};

use log::debug;
use futures::TryFutureExt;
use slug::slugify;
use url::Url;

use config::{
    dependency::{Dependency, DependencyTarget},
    plugin::{Plugin, PluginMap, PluginSource},
    registry::RegistryItem,
    semver::{Version, VersionReq},
    PLUGIN,
};

use crate::{
    archive::reader::PackageReader, compute, download, reader::read, Error,
    Registry, Result,
};

static GIT_SCHEME: &str = "scm";
static FILE_SCHEME: &str = "file";

pub async fn install<P: AsRef<Path>>(
    project: P,
    registry: &Registry<'_>,
    dep: &Dependency,
    locals: Option<PluginMap>,
) -> Result<Plugin> {
    let plugin = if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => {
                install_path(project, path, None).await
            }
            DependencyTarget::Archive { ref archive } => {
                install_archive(project, archive).await
            }
            DependencyTarget::Repo { ref git } => {
                install_repo(project, git).await
            }
            DependencyTarget::Local { ref scope } => {
                install_local(project, scope, locals).await
            }
        }
    } else {
        install_registry(project, registry, dep).await
    }?;

    Ok(plugin)
}

/// Resolve to a canonical path.
fn canonical<P: AsRef<Path>, F: AsRef<Path>>(
    project: P,
    path: F,
) -> Result<PathBuf> {
    let mut file = path.as_ref().to_path_buf();
    if !path.as_ref().is_absolute() {
        file = project
            .as_ref()
            .canonicalize()
            .unwrap_or(project.as_ref().to_path_buf())
            .join(path.as_ref())
            .canonicalize()
            .unwrap_or(path.as_ref().to_path_buf());
    }
    Ok(file)
}

/// Install a plugin from a file system path.
async fn install_file<P: AsRef<Path>, F: AsRef<Path>>(
    project: P,
    path: F,
) -> Result<(PathBuf, Plugin)> {
    let file = canonical(project, path)?;

    if !file.exists() || !file.is_dir() {
        return Err(Error::PluginPathNotDirectory(file));
    }

    let plugin = read(&file).await?;
    Ok((file, plugin))
}

/// Install a plugin from a file system path and compute the
/// plugin data.
pub async fn install_path<P: AsRef<Path>, F: AsRef<Path>>(
    project: P,
    path: F,
    source: Option<PluginSource>,
) -> Result<Plugin> {
    let (target, mut plugin) = install_file(project.as_ref(), path).await?;

    //let url_target =
    //format!("{}:{}", FILE_SCHEME, utils::url::to_href_separator(&target));
    //let source: Url = url_target.parse()?;

    let source = if let Some(ref source) = source {
        source.clone()
    } else {
        PluginSource::File(target.to_path_buf())
    };

    attributes(&mut plugin, &target, source, None)?;

    compute::transform(&plugin).await
}

/// Install from a local archive file.
///
/// No expected digest is available so this method should be treated with caution
/// and only used with packages created using the `plugin pack` command.
pub async fn install_archive<P: AsRef<Path>, F: AsRef<Path>>(
    project: P,
    path: F,
) -> Result<Plugin> {
    // Determine the location to extract the archive to.
    let builder =
        |_: &PathBuf, plugin: &Plugin, digest: &Vec<u8>| -> Result<PathBuf> {
            let name = format!(
                "{}{}{}{}{}{}{}",
                config::PLUGIN_ARCHIVE_PREFIX,
                config::PLUGIN_NS,
                &plugin.name,
                config::PLUGIN_NS,
                plugin.version.to_string(),
                config::PLUGIN_NS,
                hex::encode(digest),
            );

            Ok(config::plugins_dir()?.join(name))
        };

    let file = canonical(project, path)?;

    // Extract the archive
    let reader =
        PackageReader::new(file.clone(), None, Some(Box::new(builder)))
            .destination(PathBuf::from("."))?
            .set_overwrite(true)
            .digest()
            .and_then(|b| b.xz())
            .and_then(|b| b.tar())
            .await?;

    let (target, digest, mut plugin) = reader.into_inner();

    let source = PluginSource::Archive(file.to_path_buf());

    //let url_target = format!("tar:{}", utils::url::to_href_separator(&file));
    //let source: Url = url_target.parse()?;
    attributes(&mut plugin, &target, source, Some(&hex::encode(digest)))?;
    Ok(plugin)
}

pub async fn install_repo<P: AsRef<Path>>(
    project: P,
    scm_url: &Url,
) -> Result<Plugin> {
    // TODO: ensure the plugin source is "scm+file" scheme

    let scheme = scm_url.scheme();
    if scheme == FILE_SCHEME {
        let path = urlencoding::decode(scm_url.path())?;
        let repo_path = Path::new(&path);
        let _ = scm::open(&repo_path)?;
        let source = Some(PluginSource::File(repo_path.to_path_buf()));
        return install_path(project, &repo_path, source).await;
    }

    let host = if let Some(host) = scm_url.host_str() {
        host
    } else {
        config::HOST
    };

    let base = config::plugins_dir()?;
    let scm_url_str = format!(
        "{}{}{}-{}",
        GIT_SCHEME,
        config::PLUGIN_NS,
        slugify(host),
        slugify(urlencoding::decode(scm_url.path())?)
    );

    let scm_target = base.join(scm_url_str);

    let _ = if scm_target.exists() && scm_target.is_dir() {
        let repo = scm::open(&scm_target)?;
        scm::pull(&scm_target, None, None)?;
        repo
    } else {
        scm::clone(&scm_url, &scm_target)?
    };

    let source = Some(PluginSource::Repo(scm_url.clone()));
    return install_path(project, &scm_target, source).await;
}

pub(crate) async fn install_local<P: AsRef<Path>, S: AsRef<str>>(
    _project: P,
    scope: S,
    locals: Option<PluginMap>,
) -> Result<Plugin> {
    let scope = scope.as_ref();
    if let Some(ref collection) = locals {
        if let Some(plugin) = collection.get(scope) {
            return Ok(plugin.clone());
        } else {
            Err(Error::PluginScopeNotFound(scope.to_string()))
        }
    } else {
        Err(Error::PluginWithNoParentScope(scope.to_string()))
    }
}

pub(crate) fn inherit(
    local_dep: &mut Dependency,
    local_plugin: &mut Plugin,
    parent_plugin: &Plugin,
    parent_dep: &Dependency,
) -> Result<()> {
    // FIXME: ensure we are using the local name only...
    //

    local_dep.apply = parent_dep.apply.clone();

    local_plugin.set_source(PluginSource::Local(local_plugin.name.clone()));
    local_plugin.set_base(parent_plugin.base().clone());
    Ok(())
}

pub(crate) async fn resolve_package(
    registry: &Registry<'_>,
    name: &str,
    version: &VersionReq,
) -> Result<(Version, RegistryItem)> {
    let entry = registry
        .entry(name)
        .await?
        .ok_or_else(|| Error::RegistryPackageNotFound(name.to_string()))?;

    let (version, package) = entry.find(version).ok_or_else(|| {
        Error::RegistryPackageVersionNotFound(
            name.to_string(),
            version.to_string(),
        )
    })?;

    Ok((version.clone(), package.clone()))
}

pub async fn dependency_installed<P: AsRef<Path>>(
    project: P,
    registry: &Registry<'_>,
    dep: &Dependency,
) -> Result<Option<Plugin>> {
    let name = dep.name();
    let (version, package) =
        resolve_package(registry, name, &dep.version).await?;
    version_installed(project, registry, name, &version, Some(&package)).await
}

pub async fn version_installed<P: AsRef<Path>>(
    project: P,
    registry: &Registry<'_>,
    name: &str,
    version: &Version,
    mut package: Option<&RegistryItem>,
) -> Result<Option<Plugin>> {
    //let name = dep.name.as_ref().unwrap();

    let extract_target = installation_dir(name, &version)?;
    let extract_target_plugin = extract_target.join(PLUGIN);

    // Got an existing plugin file in the target cache directory
    // so we should try to use that
    if extract_target_plugin.exists() {
        let (target, mut plugin) =
            install_file(project, &extract_target).await?;
        let source = PluginSource::Registry(download::REGISTRY.parse()?);

        let package = if let Some(package) = package.take() {
            package.clone()
        } else {
            let (_, package) =
                resolve_package(registry, name, &VersionReq::exact(version))
                    .await?;
            package
        };

        attributes(&mut plugin, &target, source, Some(&package.digest))?;
        return Ok(Some(plugin));
    }

    Ok(None)
}

pub fn installation_dir(name: &str, version: &Version) -> Result<PathBuf> {
    let extract_dir =
        format!("{}{}{}", name, config::PLUGIN_NS, version.to_string());
    Ok(config::plugins_dir()?.join(extract_dir))
}

/// Assign some private attributes to the plugin.
fn attributes(
    plugin: &mut Plugin,
    base: &PathBuf,
    source: PluginSource,
    digest: Option<&str>,
) -> Result<()> {
    plugin.set_base(base);
    plugin.set_source(source);
    if let Some(digest) = digest {
        plugin.set_checksum(digest);
    }
    Ok(())
}

/// Install a plugin using the local registry cache and archives
/// from an online service (s3 bucket).
///
/// The registry stores plugin definitions by namespace such as `std::core.json`
/// which references the versions available for a plugin.
///
/// Once we have a registry entry we attempt to download the archive from the
/// bucket using the path `std::core/1.0.0/package.xz`.
///
/// Finally we extract the downloaded archive and verify the digest from the registry
/// entry to a local file system cache directory `cache/src/std::core::1.0.0` within
/// the main program home directory, currently `~/.uwe`.
pub async fn install_registry<P: AsRef<Path>>(
    project: P,
    registry: &Registry<'_>,
    dep: &Dependency,
) -> Result<Plugin> {
    let name = dep.name.as_ref().unwrap();
    let (version, package) =
        resolve_package(registry, name, &dep.version).await?;

    let extract_target = installation_dir(name, &version)?;

    if let Some(plugin) =
        dependency_installed(project, registry, dep).await?.take()
    {
        return Ok(plugin);
    }

    // We will extract the temporary archive file here so the
    // directory must exist
    if !extract_target.exists() {
        fs::create_dir(&extract_target)?;
    }

    let fetch_info = download::get(name, &version).await?;

    debug!("Extracting archive {}", fetch_info.archive.display());
    let reader = PackageReader::new(
        fetch_info.archive.to_path_buf(),
        Some(hex::decode(&package.digest)?),
        None,
    )
    .destination(&extract_target)?
    .digest()
    .and_then(|b| b.xz())
    .and_then(|b| b.tar())
    .await?;

    let (_target, _digest, mut plugin) = reader.into_inner();
    let source = PluginSource::Registry(download::REGISTRY.parse()?);
    attributes(&mut plugin, &extract_target, source, Some(&package.digest))?;
    Ok(plugin)
}
