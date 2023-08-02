use std::fs::File;
use std::io::stderr;
use std::path::PathBuf;

use http::StatusCode;
use human_bytes::human_bytes;
use log::debug;
use pbr::{ProgressBar, Units};
use semver::Version;
use tokio::io::AsyncWriteExt;

use crate::{Error, Result};

pub(crate) const REGISTRY: &str =
    "http://s3-ap-southeast-1.amazonaws.com/registry.uwe.app";

#[derive(Debug)]
pub struct FetchInfo {
    pub archive: PathBuf,
    pub url: String,
    pub cached: bool,
}

fn local_archive(name: &str, version: &Version) -> Result<PathBuf> {
    let downloads_cache_dir = dirs::downloads_dir()?;
    let downloads_cache_name =
        format!("{}{}{}.tar.xz", name, config::PLUGIN_NS, version);
    Ok(downloads_cache_dir.join(&downloads_cache_name))
}

fn remote_url(name: &str, version: &Version) -> String {
    format!(
        "{}/{}/{}/{}.tar.xz",
        REGISTRY,
        name,
        version.to_string(),
        config::PACKAGE
    )
}

/// Get a plugin archive either from the download cache if it exists otherwise
/// try to download and cache the archive.
pub async fn get(name: &str, version: &Version) -> Result<FetchInfo> {
    let archive = local_archive(name, version)?;
    if archive.exists() && archive.is_file() {
        let url = remote_url(name, version);
        return Ok(FetchInfo {
            archive,
            url,
            cached: true,
        });
    }
    fetch(name, version).await
}

/// Download a plugin archive from an online source such as an s3 bucket
/// into the downloads cache directory.
async fn fetch(name: &str, version: &Version) -> Result<FetchInfo> {
    let archive = local_archive(name, version)?;
    let url = remote_url(name, version);

    log::info!("download {}", url);

    let dest = File::create(&archive)?;

    let mut response = reqwest::get(&url).await?;
    if response.status() != StatusCode::OK {
        return Err(Error::RegistryDownloadFail(
            response.status().to_string(),
            url,
        ));
    }

    let len = response.content_length().unwrap_or(0);
    log::info!("remote length {}", len);

    let mut pb = ProgressBar::on(stderr(), len);
    pb.set_units(Units::Bytes);
    pb.show_speed = false;
    let msg = format!(" Fetch {}@{} ", name, version);
    pb.message(&msg);

    let mut content_file = tokio::fs::File::from_std(dest);
    while let Some(chunk) = response.chunk().await? {
        pb.add(chunk.len() as u64);
        content_file.write_all(&chunk).await?;
    }

    content_file.flush().await?;

    let meta = tokio::fs::metadata(&archive).await?;
    log::info!("file length {}", meta.len());

    let msg = format!(
        " Fetched {}@{} ({})",
        name,
        version,
        human_bytes(len as f64)
    );
    pb.finish_print(&msg);

    Ok(FetchInfo {
        archive,
        url,
        cached: false,
    })
}
