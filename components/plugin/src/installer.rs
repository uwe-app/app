use std::fs::{self, File};
use std::path::{Path, PathBuf};

use futures::TryFutureExt;
use tokio::prelude::*;
use http::StatusCode;
use url::Url;
use log::info;
use slug::slugify;

use config::{
    dependency::{Dependency, DependencyTarget},
    registry::RegistryItem,
    semver::{Version, VersionReq},
    Plugin, PLUGIN,
};

use crate::{
    archive::reader::PackageReader,
    reader::read, compute, Error, Registry, Result,
};

//static REGISTRY: &str = "https://registry.hypertext.live";
static REGISTRY: &str = "https://s3-ap-southeast-1.amazonaws.com/registry.hypertext.live";

static GIT_SCHEME: &str = "git";
static FILE_SCHEME: &str = "file";

pub async fn install(
    registry: &Registry<'_>,
    dep: &Dependency,
) -> Result<Plugin> {
    let plugin = if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => install_path(path).await,
            DependencyTarget::Archive { ref archive } => {
                install_archive(archive).await
            }
            DependencyTarget::Repo { ref git } => {
                install_repo(git).await
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

/// Install a plugin from a file system path and compute the 
/// plugin data.
pub(crate) async fn install_path<P: AsRef<Path>>(path: P) -> Result<Plugin> {
    let mut plugin = install_file(path.as_ref()).await?;

    let target = path.as_ref().to_path_buf();
    let canonical = path.as_ref().canonicalize()?;
    let url_target = format!(
        "{}:{}",
        FILE_SCHEME,
        utils::url::to_href_separator(&canonical));
    let source: Url = url_target.parse()?;
    attributes(&mut plugin, &target, source, None)?;

    compute::transform(&plugin).await
}

/// Install from a local archive file.
///
/// No expected digest is available so this method should be treated with caution 
/// and only used with packages created using the `plugin pack` command.
pub(crate) async fn install_archive<P: AsRef<Path>>(path: P) -> Result<Plugin> {

    // Determine the location to extract the archive to.
    let builder = |_: &PathBuf, plugin: &Plugin, digest: &Vec<u8>| -> Result<PathBuf> {
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

        Ok(cache::get_cache_src_dir()?.join(name))
    };

    // Extract the archive
    let reader = PackageReader::new(path.as_ref().to_path_buf(), None, Some(Box::new(builder)))
        .destination(PathBuf::from("."))?
        .set_overwrite(true)
        .digest()
        .and_then(|b| b.xz())
        .and_then(|b| b.tar())
        .await?;

    let (target, digest, mut plugin) = reader.into_inner();
    let canonical = path.as_ref().canonicalize()?;
    let url_target = format!(
        "tar:{}",
        utils::url::to_href_separator(&canonical));
    let source: Url = url_target.parse()?;
    attributes(&mut plugin, &target, source, Some(&hex::encode(digest)))?;
    Ok(plugin)
}

pub(crate) async fn install_repo<S: AsRef<str>>(git: S) -> Result<Plugin> {
    let git_url: Url = git.as_ref().parse().map_err(|e| Error::GitUrl(e))?;

    // TODO: ensure the plugin source is "git+file" scheme

    let scheme = git_url.scheme();
    if scheme == FILE_SCHEME {
        let path = urlencoding::decode(git_url.path())?;
        let repo_path = Path::new(&path);
        let _ = git::open_repo(&repo_path)?;
        return install_path(&repo_path).await
    }

    let host = if let Some(host) = git_url.host_str() {
        host
    } else { config::HOST };

    let base = cache::get_cache_src_dir()?;
    let git_url_str = format!(
        "{}{}{}-{}",
        GIT_SCHEME,
        config::PLUGIN_NS,
        slugify(host),
        slugify(urlencoding::decode(git_url.path())?));

    let git_target = base.join(git_url_str);

    let _ = if git_target.exists() && git_target.is_dir() {
        let repo = git::open_repo(&git_target)?;
        git::pull(&git_target, None, None)?;
        repo
    } else {
        git::clone(&git_url, &git_target)? 
    };

    return install_path(&git_target).await
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
        let source: Url = REGISTRY.parse()?;
        attributes(&mut plugin, &extract_target, source, Some(&package.digest))?;
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
fn attributes(plugin: &mut Plugin, base: &PathBuf, source: Url, digest: Option<&str>) -> Result<()> {
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
    let file_name = format!("{}.tar.xz", config::PACKAGE);
    let download_url = format!(
        "{}/{}/{}/{}.tar.xz",
        REGISTRY,
        name,
        version.to_string(),
        config::PACKAGE
    );

    info!("Download {}", download_url);

    let archive_path = download_dir.path().join(&file_name);
    let dest = File::create(&archive_path)?;

    let mut response = reqwest::get(&download_url).await?;
    if response.status() != StatusCode::OK {
        return Err(
            Error::RegistryDownloadFail(response.status().to_string(), download_url));
    }

    //let len = response.content_length().unwrap_or(0u64);
    //println!("Expected content length {}", len);

    // FIXME: show progress bar for download (#220)

    let mut content_file = tokio::fs::File::from_std(dest);
    let mut bytes_read = 0usize;
    while let Some(chunk) = response.chunk().await? {
        content_file.write_all(&chunk).await?;
        bytes_read += chunk.len();
        info!("Downloaded {} bytes", bytes_read);
    }

    //println!("Downloaded {:?} bytes", content_file.metadata().await?.len());
    //println!("Downloaded {:?} bytes", File::open(&archive_path)?.metadata()?.len());

    let reader =
        PackageReader::new(archive_path, Some(hex::decode(&package.digest)?), None)
            .destination(&extract_target)?
            .digest()
            .and_then(|b| b.xz())
            .and_then(|b| b.tar())
            .await?;

    let (_target, _digest, mut plugin) = reader.into_inner();
    let source: Url = REGISTRY.parse()?;
    attributes(&mut plugin, &extract_target, source, Some(&package.digest))?;
    Ok(plugin)
}

