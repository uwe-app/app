use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, stderr, stdout, Write};
use std::path::PathBuf;

use crossterm::{
    cursor::MoveUp,
    execute,
    terminal::{Clear, ClearType},
};
use human_bytes::human_bytes;
use pbr::{ProgressBar, Units};
use sha3::{Digest, Sha3_256};
use tempfile::NamedTempFile;

use log::{debug, info};
use semver::Version;
use url::Url;

use http::StatusCode;

use crate::{
    releases::{self, ReleaseInfo},
    Error, Result,
};

static RELEASE_URL: &str = "https://releases.uwe.app";
//static RELEASE_URL: &str = "http://releases.uwe.app.s3-website-ap-southeast-1.amazonaws.com";

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
    info: &ReleaseInfo,
    names: &[&str],
) -> Result<HashMap<String, PathBuf>> {
    let version_dir = releases::dir(version)?;

    if !version_dir.exists() {
        fs::create_dir_all(&version_dir)?;
    }

    let platform_info =
        info.platforms.get(&releases::current_platform()).unwrap();

    let mut output: HashMap<String, PathBuf> = HashMap::new();

    info!("Download {} components: {}", names.len(), names.join(", "));

    //for name in releases::INSTALL_EXE_NAMES.iter() {
    for name in names.iter() {
        let expected = platform_info.get(*name).unwrap();

        //info!("Download {}@{}", name, version.to_string());
        let url = url(version, name)?;
        let download_file = version_dir.join(name);

        debug!("Download {}", url.to_string());
        debug!("File {}", download_file.display());

        let temp_target = NamedTempFile::new()?;
        let mut temp_download = temp_target.reopen()?;

        let checksum = download(&url, &mut temp_download, name).await?;
        let received = hex::encode(&checksum);

        if received != expected.hex() {
            return Err(Error::DigestMismatch(
                name.to_string(),
                expected.hex(),
                received,
            ));
        }

        // Remove any existing target
        if download_file.exists() {
            fs::remove_file(&download_file)?;
        }

        // Copy the temporary download into place, cannot use fs::rename()
        // as tempfile() does not yield a path!
        let mut install_file = File::create(&download_file)?;
        let mut temp_source = temp_target.reopen()?;
        io::copy(&mut temp_source, &mut install_file)?;

        output.insert(name.to_string(), download_file);
    }

    let mut stderr = stderr();
    execute!(stderr, Clear(ClearType::CurrentLine))?;

    let mut stdout = stdout();
    execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine), MoveUp(1))?;

    // HACK: so future log messages are aligned correctly
    stdout.write(" ".as_bytes())?;
    stdout.flush()?;

    Ok(output)
}

/// Download a single artifact.
async fn download(
    url: &Url,
    content_file: &mut File,
    name: &str,
) -> Result<Vec<u8>> {
    let mut response = reqwest::get(url.clone()).await?;
    if response.status() != StatusCode::OK {
        return Err(Error::DownloadFail(
            response.status().to_string(),
            url.to_string(),
        ));
    }

    let len = response.content_length().unwrap_or(0);

    let mut pb = ProgressBar::on(stderr(), len);
    pb.set_units(Units::Bytes);
    pb.show_speed = false;
    let msg = format!(" Downloading {}(1) ", name);
    pb.message(&msg);

    let mut hasher = Sha3_256::new();

    while let Some(chunk) = response.chunk().await? {
        content_file.write_all(&chunk)?;
        pb.add(chunk.len() as u64);

        let mut bytes: &[u8] = chunk.as_ref();
        std::io::copy(&mut bytes, &mut hasher)?;
    }

    let msg = format!(" Downloaded {} ({})", name, human_bytes(len as f64));
    pb.finish_print(&msg);

    Ok(hasher.finalize().as_slice().to_owned())
}
