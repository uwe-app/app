use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use log::{debug, info};
use semver::Version;
use url::Url;

use http::StatusCode;

use crate::{
    checksum,
    releases::{self, ReleaseVersion},
    Error, Result,
};

static RELEASE_URL: &str = "https://release.uwe.app";
//static RELEASE_URL: &str = "http://release.uwe.app.s3-website-ap-southeast-1.amazonaws.com";

pub(crate) fn url(version: &Version, name: &str) -> Result<Url> {
    let full_url = format!(
        "{}/{}/{}/{}",
        RELEASE_URL,
        version.to_string(),
        releases::current_platform(),
        name
    );

    Ok(full_url.parse()?)
}

/// Download all the artifacts for a version and
/// verify that the checksums match.
pub(crate) async fn all(
    version: &Version,
    info: &ReleaseVersion,
    names: &[&str],
) -> Result<HashMap<String, PathBuf>> {
    let version_dir = releases::dir(version)?;

    if !version_dir.exists() {
        fs::create_dir_all(&version_dir)?;
    }

    let platform_info =
        info.platforms.get(&releases::current_platform()).unwrap();

    let mut output: HashMap<String, PathBuf> = HashMap::new();

    //for name in releases::INSTALL_EXE_NAMES.iter() {
    for name in names.iter() {
        let expected = platform_info.get(*name).unwrap();

        //info!("Download {}@{}", name, version.to_string());
        let url = url(version, name)?;
        let download_file = version_dir.join(name);
        info!("Download {}", url.to_string());
        debug!("File {}", download_file.display());

        download(&url, &download_file).await?;

        debug!("Verify checksum {}", download_file.display());
        let received = hex::encode(checksum::digest(&download_file)?);
        if &received != expected {
            return Err(Error::DigestMismatch(
                name.to_string(),
                expected.clone(),
                received,
            ));
        }

        output.insert(name.to_string(), download_file);
    }

    Ok(output)
}

/// Download a single artifact.
pub(crate) async fn download<P: AsRef<Path>>(url: &Url, path: P) -> Result<()> {
    let mut response = reqwest::get(url.clone()).await?;
    if response.status() != StatusCode::OK {
        return Err(Error::DownloadFail(
            response.status().to_string(),
            url.to_string(),
        ));
    }

    // FIXME: show progress bar for download (#220)

    let mut content_file = File::create(path.as_ref())?;
    let mut bytes_read = 0usize;
    while let Some(chunk) = response.chunk().await? {
        content_file.write_all(&chunk)?;
        bytes_read += chunk.len();
        info!("Downloaded {} bytes", bytes_read);
    }

    Ok(())
}
