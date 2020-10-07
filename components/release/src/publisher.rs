use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use semver::Version;
use log::{info, warn};

use crate::{Error, Result, checksum, releases};

/// WARN: Assumes we are building on linux!

static NAMES: [&str; 2] = ["uwe", "upm"];
static LINUX: &str = "linux";
static MACOS: &str = "macos";

static BUCKET: &str = "release.uwe.app";
static PROFILE: &str = "uwe";

static LINUX_PREFIX: &str = "target/release";
static MACOS_PREFIX: &str = "target/x86_64-apple-darwin/release";
//static WINDOWS: &str = "windows";

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
        (LINUX.to_string(), cwd.join(LINUX_PREFIX)),
        (MACOS.to_string(), cwd.join(MACOS_PREFIX))
    ];

    for (platform_name, target_dir) in platform_targets.into_iter() {
        let artifacts = executables.entry(platform_name)
            .or_insert(HashMap::new());
        for name in NAMES.iter() {
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
        .entry(semver)
        .or_insert(Default::default());

    for (platform, artifacts) in artifacts.into_iter() {
        let release_artifacts = release_versions.platforms
            .entry(platform) 
            .or_insert(Default::default());

        for (name, info) in artifacts.into_iter() {
            release_artifacts.insert(name, hex::encode(info.digest)); 
        }
    }

    //info!("{:#?}", releases);

    info!("Save {}", releases_file.display());
    releases::save(&releases_file, &releases)?;

    Ok(())
}
