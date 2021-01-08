use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use human_bytes::human_bytes;
use log::{info, warn};
use semver::Version;

use crate::{
    checksum,
    releases::{self, ReleaseVersion, ExecutableTargets, ExecutableArtifact},
    Error, Result,
};

/// WARN: Assumes we are building on linux!

static INSTALL_SH: &str = "install.sh";
static LINUX_PREFIX: &str = "target/release";
static MACOS_PREFIX: &str = "target/x86_64-apple-darwin/release";

/// Create a release build.
fn build(cwd: &PathBuf) -> Result<()> {
    let mut command = Command::new("make");
    let tasks = vec!["build-release", "build-linux-macos-cross"];
    command.current_dir(cwd).args(tasks);
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    command.output()?;
    Ok(())
}

/// Gather the build artifacts.
fn artifacts(cwd: &PathBuf) -> Result<ExecutableTargets> {
    let mut executables = BTreeMap::new();

    let platform_targets = vec![
        (releases::LINUX.to_string(), cwd.join(LINUX_PREFIX)),
        (releases::MACOS.to_string(), cwd.join(MACOS_PREFIX)),
    ];

    for (platform_name, target_dir) in platform_targets.into_iter() {
        let artifacts =
            executables.entry(platform_name).or_insert(BTreeMap::new());
        for name in releases::PUBLISH_EXE_NAMES.iter() {
            let path = target_dir.join(name);

            if !path.exists() || !path.is_file() {
                return Err(Error::NoBuildArtifact(path.to_path_buf()));
            }

            info!("Calculate digest {}", path.display());

            let digest = checksum::digest(&path)?;
            let artifact = ExecutableArtifact { path, digest };
            artifacts.insert(name.to_string(), artifact);
        }
    }

    Ok(executables)
}

fn get_key(version: &str, platform: &str, name: &str) -> String {
    format!("{}/{}/{}", version, platform, name)
}

/// Upload the plugin package to the s3 bucket.
async fn upload(
    file: &PathBuf,
    bucket: &str,
    region: &str,
    profile: &str,
    version: &Version,
    platform: &str,
    name: &str,
) -> Result<()> {
    let aws_region = publisher::parse_region(region)?;

    let key = get_key(&version.to_string(), platform, name);

    let bytes = file.metadata()?.len();

    info!(
        "Upload {} to {}/{} ({})",
        human_bytes(bytes as f64),
        bucket,
        key,
        region
    );
    publisher::put_file(file, &key, aws_region, bucket, profile).await?;
    info!("{} ✓", &key);

    Ok(())
}

/// Configure redirects from `latest` to the new version.
async fn redirects(
    bucket: &str,
    region: &str,
    profile: &str,
    version: &Version,
    release: &ReleaseVersion,
) -> Result<()> {
    let aws_region = publisher::parse_region(region)?;
    for (platform, targets) in release.platforms.iter() {
        for (name, _) in targets {
            let key = get_key(releases::LATEST, platform, name);
            let location =
                format!("/{}", get_key(&version.to_string(), platform, name));
            info!("Redirect {} -> {}", key, location);
            publisher::put_redirect(
                &location,
                &key,
                aws_region.clone(),
                bucket,
                profile,
            )
            .await?;
        }
    }

    Ok(())
}

/// Upload the quick install script.
async fn script(
    bucket: &str,
    region: &str,
    profile: &str,
    project: &PathBuf,
) -> Result<()> {
    let aws_region = publisher::parse_region(region)?;
    let file = releases::local_releases(project)?.join(INSTALL_SH);
    let key = INSTALL_SH.to_string();
    info!("Upload install script {}", INSTALL_SH);
    publisher::put_file(&file, &key, aws_region, bucket, profile).await?;
    Ok(())
}

/// Publish all the release artifacts.
///
/// 1) Compile the release artifacts.
/// 2) Upload all the release executables.
/// 3) Update the release registry index.
///
pub async fn publish(
    manifest: String,
    name: String,
    version: String,
    bucket: String,
    region: String,
    profile: String,
    skip_build: bool,
    skip_upload: bool,
    force_overwrite: bool,
) -> Result<()> {
    info!("Release {}@{}", &name, &version);

    let semver: Version = version.parse()?;
    let manifest = PathBuf::from(manifest).canonicalize()?;
    let releases_repo = releases::local_releases(&manifest)?;
    let releases_file = releases::local_manifest_file(&manifest)?;

    let mut releases = releases::load(&releases_file)?;
    if releases.versions.contains_key(&semver) {
        if !force_overwrite {
            return Err(Error::ReleaseVersionExists(semver.to_string()));
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
    let release_versions = releases
        .versions
        .entry(semver.clone())
        .or_insert(Default::default());

    for (platform, artifacts) in artifacts.into_iter() {
        let release_artifacts = release_versions
            .platforms
            .entry(platform.clone())
            .or_insert(Default::default());

        for (name, info) in artifacts.into_iter() {

            if !skip_upload {
                upload(
                    &info.path, &bucket, &region, &profile, &semver, &platform,
                    &name,
                )
                .await?;
            }

            release_artifacts.insert(name, hex::encode(info.digest));
        }
    }

    // Set up the website redirects for latest.
    redirects(&bucket, &region, &profile, &semver, &release_versions).await?;

    // TODO: invalidate the redirect paths in cloudfront!!!

    // Upload the quick curl install script.
    script(&bucket, &region, &profile, &manifest).await?;

    info!("Save {}", releases_file.display());
    releases::save(&releases_file, &releases)?;

    // Commit and push the release manifest
    let repo = scm::open(&releases_repo)?;
    info!("Commit releases manifest {}", releases_file.display());
    scm::commit_file(&repo, Path::new(releases::MANIFEST_JSON), "Update release manifest.")?;
    info!("Push {}", releases_repo.display());
    scm::push_remote_name(&repo, scm::ORIGIN, None, None)?;

    Ok(())
}
