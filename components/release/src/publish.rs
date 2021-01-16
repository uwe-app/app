use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use human_bytes::human_bytes;
use log::{info, warn};
use semver::Version;

use crate::{
    checksum,
    releases::{self, ExecutableArtifact, ExecutableTargets, ReleaseInfo},
    Error, Result,
};

use config::plugin::{Plugin, VersionKey};

/// WARN: Assumes we are building on linux!

static INSTALL_SH: &str = "install.sh";
static LINUX_PREFIX: &str = "target/release";
static MACOS_PREFIX: &str = "target/x86_64-apple-darwin/release";

static UWE_BINARY: &str = "target/release/uwe";
static UPM_BINARY: &str = "target/release/upm";

/// Publish a new release.
///
/// - Compile the release artifacts.
/// - Upload all the release executables.
/// - Update the `latest` redirects to point to the new version.
/// - Upload the quick install script.
/// - Update the release registry index.
/// - Push the releases repository with the updated manifest file.
/// - Publish the stage.uwe.app website
/// - Build the offline documentation
/// - Copy the release manifest to the releases.uwe.app website source.
/// - Commit and push the website releases repository with the updated manifest.
/// - Publish the website for releases.uwe.app using the new manifest.
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
    let release_version = VersionKey::from(&semver);
    if releases.versions.contains_key(&release_version) {
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
        .entry(release_version)
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

            release_artifacts.insert(name, info);
        }
    }

    // Set up the website redirects for latest.
    latest_redirects(&bucket, &region, &profile, &semver, &release_versions)
        .await?;

    // TODO: invalidate the redirect paths in cloudfront!!!

    // Upload the quick curl install script.
    upload_quick_install_script(&bucket, &region, &profile, &manifest).await?;

    info!("Save {}", releases_file.display());
    releases::save(&releases_file, &releases)?;

    // Commit and push the release manifest
    let repo = scm::open(&releases_repo)?;
    info!("Commit releases manifest {}", releases_file.display());
    scm::commit_file(
        &repo,
        Path::new(releases::MANIFEST_JSON),
        "Update release manifest.",
    )?;
    info!("Push {}", releases_repo.display());
    scm::push_remote_name(&repo, scm::ORIGIN, None, None)?;

    let website_repo = PathBuf::from("../sites/website");
    update_website(&website_repo)?;
    update_releases_website(&releases_file)?;

    update_documentation(&website_repo, &semver)?;

    Ok(())
}

fn update_website(website_repo: &PathBuf) -> Result<()> {
    // FIXME: do not remove the lock file!
    let lock_file = website_repo.join(config::SITE_LOCK);
    fs::remove_file(&lock_file)?;
    publish_website(&website_repo, "stage")?;
    Ok(())
}

/// Update the offline documentation.
///
/// Note we do not use the `make` tasks to compile as we need to be certain we
/// are using the version of `uwe(1)` that we just compiled.
fn update_documentation(
    website_repo: &PathBuf,
    version: &Version,
) -> Result<()> {
    Command::new(UWE_BINARY)
        .args(vec![
            "build",
            "--profile",
            "docs",
            &website_repo.to_string_lossy(),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    let build_files = website_repo.join("build").join("docs");
    let documentation_repo = PathBuf::from("../documentation");
    let public_html = documentation_repo.join("public_html");

    fs::remove_dir_all(&public_html)?;

    // Copy over the build/docs directory from the website
    Command::new("cp")
        .args(vec![
            "-rf",
            &build_files.to_string_lossy(),
            &documentation_repo.to_string_lossy(),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    // Move files into place as `public_html`
    let documentation_target = documentation_repo.join("docs");
    Command::new("mv")
        .args(vec![
            "-f",
            &documentation_target.to_string_lossy(),
            &public_html.to_string_lossy(),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    // Update the plugin version and write the new version to disc
    let plugin_file = documentation_repo.join(config::PLUGIN);
    info!(
        "Write plugin version {} to {}",
        &version,
        plugin_file.display()
    );
    let plugin_content = fs::read_to_string(&plugin_file)?;
    let mut plugin: Plugin = toml::from_str(&plugin_content)?;
    plugin.set_version(version.clone());
    fs::write(&plugin_file, toml::to_vec(&plugin)?)?;

    // Commit and push the documentation repo
    Command::new("make")
        .args(vec!["push"])
        .current_dir(&documentation_repo)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    // Publish the plugin with the new version
    Command::new(UPM_BINARY)
        .args(vec!["publish", &documentation_repo.to_string_lossy()])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    // TODO: launch a server with the documentation for verification?

    Ok(())
}

/// Create a release build.
fn build(cwd: &PathBuf) -> Result<()> {
    Command::new("make")
        .current_dir(cwd)
        .args(vec!["build-release", "build-linux-macos-cross"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
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
            let meta = std::fs::metadata(&path)?;
            let size = meta.len();
            let artifact = ExecutableArtifact { path, digest, size };

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
async fn latest_redirects(
    bucket: &str,
    region: &str,
    profile: &str,
    version: &Version,
    release: &ReleaseInfo,
) -> Result<()> {
    let aws_region = publisher::parse_region(region)?;
    for (platform, targets) in release.platforms.iter() {
        for (name, _) in targets {
            let key = get_key(releases::LATEST, platform, name);
            let location =
                format!("/{}", get_key(&version.to_string(), platform, name));
            info!("Redirect {} -> {}", key, location);
            publisher::put_redirect_once(
                profile,
                aws_region.clone(),
                bucket,
                &key,
                &location,
            )
            .await?;
        }
    }

    Ok(())
}

/// Upload the quick install script.
async fn upload_quick_install_script(
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

/// Copy the manifest file to the releases website which should be
/// in the `../sites/releases` relative location.
///
/// Then publish the website using the `production` environment so that
/// the content for https://releases.uwe.app is also updated.
fn update_releases_website(releases_file: &PathBuf) -> Result<()> {
    let manifest_file = Path::new("site/collections/releases/manifest.json");
    let releases_website_repo = PathBuf::from("../sites/releases");
    let releases_website_manifest = releases_website_repo.join(&manifest_file);

    // Copy the release manifest to the website source for
    // the releases.uwe.app website
    if let Some(parent) = releases_website_manifest.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    std::fs::copy(&releases_file, &releases_website_manifest)?;

    // Commit and push the release manifest
    let repo = scm::open(&releases_website_repo)?;
    info!(
        "Commit manifest for releases website {}",
        releases_website_manifest.display()
    );
    scm::commit_file(&repo, manifest_file, "Update release manifest.")?;
    info!("Push {}", releases_website_repo.display());
    scm::push_remote_name(&repo, scm::ORIGIN, None, None)?;

    // Compile and publish the website
    // FIXME: do not remove the lock file!
    let lock_file = releases_website_repo.join(config::SITE_LOCK);
    fs::remove_file(&lock_file)?;
    publish_website(&releases_website_repo, "production")?;

    Ok(())
}

fn publish_website(repo: &PathBuf, environment: &str) -> Result<()> {
    let repo_path = repo.to_string_lossy().into_owned().to_string();
    let cwd = std::env::current_dir()?;
    Command::new(UWE_BINARY)
        .current_dir(cwd)
        .args(vec!["publish", environment, &repo_path])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    Ok(())
}
