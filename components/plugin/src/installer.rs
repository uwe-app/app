use std::fs::{self, File};
use std::path::{Path, PathBuf};

use futures::TryFutureExt;
use tokio::prelude::*;

use url::Url;

use config::{
    dependency::{Dependency, DependencyTarget},
    registry::RegistryItem,
    semver::{Version, VersionReq},
    Plugin, PLUGIN,
};

use crate::{
    archive::reader::PackageReader, reader::read, Error, Registry, Result,
};

static REGISTRY: &str = "https://registry.hypertext.live";

pub async fn install(
    registry: &Registry<'_>,
    dep: &Dependency,
) -> Result<Plugin> {
    let plugin = if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => install_file(path).await,
            DependencyTarget::Archive { ref archive } => {
                install_archive(archive).await
            }
        }
    } else {
        install_registry(registry, dep).await
    }?;

    Ok(plugin)
}

/// Install a plugin from a file system path.
async fn install_file<P: AsRef<Path>>(path: P) -> Result<Plugin> {
    read(path.as_ref()).await
}

/// Install from a local archive file.
///
/// No digest is available so this method is unsafe.
async fn install_archive<P: AsRef<Path>>(path: P) -> Result<Plugin> {
    let archive = path.as_ref();

    let dir = tempfile::tempdir()?;

    // FIXME: extract this to a tmp dir that can be used for the build

    // Must go into the tempdir so it is not
    // automatically cleaned up before we
    // are done with it.
    let path = dir.into_path();

    let reader = PackageReader::new(archive.to_path_buf(), None)
        .destination(&path)?
        .xz()
        .and_then(|b| b.tar())
        .await?;

    let (target, _digest, plugin) = reader.into_inner();

    println!("Archive plugin {:#?}", &plugin);
    println!("Archive plugin target {:#?}", &target);

    // Clean up the temp dir
    println!("Removing the temp archive {}", target.display());
    std::fs::remove_dir_all(target)?;

    todo!()
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

pub(crate) async fn get_cached(
    registry: &Registry<'_>,
    dep: &Dependency,
) -> Result<Option<Plugin>> {
    let name = dep.name.as_ref().unwrap();
    let (version, package) =
        resolve_package(registry, name, &dep.version).await?;

    let extract_target = get_extract_dir(name, &version)?;
    let extract_target_plugin = extract_target.join(PLUGIN);

    // Got an existing plugin file in the target cache directory
    // so we should try to use that
    if extract_target_plugin.exists() {
        let mut plugin = install_file(&extract_target).await?;
        attributes(&mut plugin, &extract_target, &package.digest)?;
        return Ok(Some(plugin));
    }

    Ok(None)
}

fn get_extract_dir(name: &str, version: &Version) -> Result<PathBuf> {
    let extract_dir =
        format!("{}{}{}", name, config::PLUGIN_NS, version.to_string());
    Ok(cache::get_cache_src_dir()?.join(extract_dir))
}

/// Assign some private attributes to the plugin.
fn attributes(plugin: &mut Plugin, base: &PathBuf, digest: &str) -> Result<()> {
    let source: Url = REGISTRY.parse()?;
    plugin.set_base(base);
    plugin.set_checksum(digest);
    plugin.set_source(source);
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
/// the main program home directory, currently `~/.hypertext`.
async fn install_registry(
    registry: &Registry<'_>,
    dep: &Dependency,
) -> Result<Plugin> {
    let name = dep.name.as_ref().unwrap();
    let (version, package) =
        resolve_package(registry, name, &dep.version).await?;

    let extract_target = get_extract_dir(name, &version)?;
    if let Some(plugin) = get_cached(registry, dep).await?.take() {
        return Ok(plugin);
    }

    // We will extract the temporary archive file here so the
    // directory must exist
    if !extract_target.exists() {
        fs::create_dir(&extract_target)?;
    }

    let download_dir = tempfile::tempdir()?;
    let file_name = format!("{}.xz", config::PACKAGE);
    let download_url = format!(
        "{}/{}/{}/{}.xz",
        REGISTRY,
        name,
        version.to_string(),
        config::PACKAGE
    );

    let archive_path = download_dir.path().join(&file_name);
    let dest = File::create(&archive_path)?;

    let mut response = reqwest::get(&download_url).await?;
    let mut content_file = tokio::fs::File::from_std(dest);
    while let Some(chunk) = response.chunk().await? {
        content_file.write_all(&chunk).await?;
    }

    let reader =
        PackageReader::new(archive_path, Some(hex::decode(&package.digest)?))
            .destination(&extract_target)?
            .digest()
            .and_then(|b| b.xz())
            .and_then(|b| b.tar())
            .await?;

    let (_target, _digest, mut plugin) = reader.into_inner();
    attributes(&mut plugin, &extract_target, &package.digest)?;
    Ok(plugin)
}
