use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use semver::Version;
use log::{info, warn};
use human_bytes::human_bytes;

use crate::{Error, Result, checksum, releases};

/// WARN: Assumes we are building on linux!

static LINUX_PREFIX: &str = "target/release";
static MACOS_PREFIX: &str = "target/x86_64-apple-darwin/release";

type Platform = String;
type ExecutableName = String;
type ExecutableTargets =
    HashMap<Platform, HashMap<ExecutableName, ExecutableArtifact>>;

#[derive(Debug)]
pub struct ExecutableArtifact {
    path: PathBuf,
    digest: Vec<u8>,
}

/// Create a release build.
fn build(cwd: &PathBuf) -> Result<()> {
    let mut command = Command::new("make");
    let tasks = vec![
        "build-release",
        "build-linux-macos-cross"
    ];
    command.current_dir(cwd).args(tasks);
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    command.output()?;
    Ok(())
}

/// Gather the build artifacts.
fn artifacts(cwd: &PathBuf) -> Result<ExecutableTargets> {
    let mut executables = HashMap::new();

    let platform_targets = vec![
        (releases::LINUX.to_string(), cwd.join(LINUX_PREFIX)),
        (releases::MACOS.to_string(), cwd.join(MACOS_PREFIX))
    ];

    for (platform_name, target_dir) in platform_targets.into_iter() {
        let artifacts = executables.entry(platform_name)
            .or_insert(HashMap::new());
        for name in releases::PUBLISH_EXE_NAMES.iter() {
            let path = target_dir.join(name);

            if !path.exists() || !path.is_file() {
                return Err(Error::NoBuildArtifact(path.to_path_buf()))
            }

            info!("Calculate digest {}", path.display());

            let digest = checksum::digest(&path)?;
            let artifact = ExecutableArtifact {path, digest};
            artifacts.insert(name.to_string(), artifact);
        }
    }

    Ok(executables)
}

/// Upload the plugin package to the s3 bucket.
async fn upload(
    file: &PathBuf,
    bucket: &str,
    region: &str,
    profile: &str,
    version: &Version,
    platform: &str,
    name: &str) -> Result<()> {

    let aws_region = publisher::parse_region(region)?;

    let key = format!(
        "{}/{}/{}",
        version.to_string(),
        platform,
        name,
    );

    let bytes = file.metadata()?.len();

    info!("Upload {} to {}/{} ({})", human_bytes(bytes as f64), bucket, key, region);
    publisher::put_file(file, &key, aws_region, bucket, profile).await?;
    info!("{} ✓", &key);

    Ok(())
}

/// Publish all the release artifacts.
///
/// 1) Compile the release artifacts.
/// 2) Upload all the release executables.
/// 3) Update the release registry index.
/// 4) Copy the installer files to the website.
///
pub async fn publish(
    manifest: String,
    name: String,
    version: String,
    bucket: String,
    region: String,
    profile: String,
    skip_build: bool,
    force_overwrite: bool,
) -> Result<()> {

    info!("Release {}@{}", &name, &version);

    let semver: Version = version.parse()?;
    let manifest = PathBuf::from(manifest).canonicalize()?;
    let releases_file = releases::repo_manifest_file(&manifest)?;

    let mut releases = releases::load(&releases_file)?;
    if releases.versions.contains_key(&semver) {
        if !force_overwrite {
            return Err(
                Error::ReleaseVersionExists(semver.to_string()));
        } else {
            warn!("Force overwrite {}", &version);
        }
    }

    if !skip_build {
        build(&manifest)?;
    } else {
        warn!("Skipping build step!");
    }

    let artifacts = artifacts(&manifest)?;
    let release_versions = releases.versions
        .entry(semver.clone())
        .or_insert(Default::default());

    for (platform, artifacts) in artifacts.into_iter() {
        let release_artifacts = release_versions.platforms
            .entry(platform.clone()) 
            .or_insert(Default::default());

        for (name, info) in artifacts.into_iter() {

            upload(
                &info.path,
                &bucket,
                &region,
                &profile,
                &semver,
                &platform,
                &name).await?;

            release_artifacts.insert(name, hex::encode(info.digest)); 
        }
    }

    //info!("{:#?}", releases);

    info!("Save {}", releases_file.display());
    releases::save(&releases_file, &releases)?;

    Ok(())
}
