use std::path::PathBuf;
use std::fs::{self, File};

use futures::TryFutureExt;
use tokio::prelude::*;

use async_recursion::async_recursion;

use config::{Dependency, DependencyMap, DependencyTarget, Plugin, PLUGIN};

use crate::{Error, PackageReader, Result, registry, registry::RegistryAccess};

static REGISTRY: &str = "https://registry.hypertext.live";

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

pub async fn read(path: &PathBuf) -> Result<Plugin> {
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

async fn load(dep: &Dependency) -> Result<Plugin> {
    if let Some(ref target) = dep.target {
        match target {
            DependencyTarget::File { ref path } => return Ok(read(&path).await?),
            DependencyTarget::Archive { ref archive } => {
                let dir = tempfile::tempdir()?;

                // FIXME: extract this to a tmp dir that can be used for the build

                // Must go into the tempdir so it is not
                // automatically cleaned up before we
                // are done with it.
                let path = dir.into_path();

                let reader = PackageReader::new(archive.clone(), None)
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
        }
    } else {
        install(dep).await
    }
}

async fn install(dep: &Dependency) -> Result<Plugin> {
    let name = dep.name.as_ref().unwrap();
    let reg = cache::get_registry_dir()?;
    let registry = registry::RegistryFileAccess::new(reg.clone(), reg.clone())?;
    let entry = registry.entry(name).await?.ok_or_else(|| {
        Error::RegistryPackageNotFound(name.to_string()) 
    })?;

    let (version, package) = entry.find(&dep.version).ok_or_else(|| {
        Error::RegistryPackageVersionNotFound(
            name.to_string(), dep.version.to_string())
    })?;

    // TODO: 1) Check if cached version of the package exists
    // TODO: 2) Fetch, cache and unpack plugin package (verify digest!)
    // TODO: 3) Load the package plugin from the file system

    let download_dir = tempfile::tempdir()?;
    let file_name = format!("{}.xz", config::PACKAGE);
    let download_url = format!("{}/{}/{}/{}.xz",
        REGISTRY, name, version.to_string(), config::PACKAGE);

    let archive_path = download_dir.path().join(&file_name);
    let dest = File::create(&archive_path)?;

    //println!("Download from {:?}", download_url);

    let mut response = reqwest::get(&download_url).await?;
    let mut content_file = tokio::fs::File::from_std(dest);
    while let Some(chunk) = response.chunk().await? {
        //println!("Chunk: {:?}", chunk.len());
        content_file.write_all(&chunk).await?;
    }

    let extract_dir = format!("{}{}{}", name, config::PLUGIN_NS, version.to_string());
    let extract_target = cache::get_cache_src_dir()?.join(extract_dir);
    if !extract_target.exists() {
        fs::create_dir(&extract_target)?;
    }

    //println!("Got downloaded file {:?}", &archive_path);
    //println!("Got extract target {:?}", extract_target);
    //println!("Got downloaded file {:?}", archive_path.metadata()?.len());

    let reader = PackageReader::new(archive_path, Some(hex::decode(&package.digest)?))
        .destination(&extract_target)?
        .digest()
        .and_then(|b| b.xz())
        .and_then(|b| b.tar())
        .await?;

    let (target, _digest, plugin) = reader.into_inner();

    todo!()
}

#[async_recursion]
pub async fn solve(
    input: DependencyMap,
    output: &mut DependencyMap,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
        dep.name = Some(name.clone());
        let mut plugin = load(&dep).await?;

        if name != plugin.name {
            return Err(Error::PluginNameMismatch(name, plugin.name));
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

        stack.push(plugin.name.clone());

        if let Some(dependencies) = plugin.dependencies.take() {
            let mut deps: DependencyMap = Default::default();
            solve(dependencies, &mut deps, stack).await?;
        }

        dep.plugin = Some(plugin);
        dep.prepare()?;

        output.items.insert(name, dep);
    }

    Ok(())
}
