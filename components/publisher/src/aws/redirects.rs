use std::fs;
use std::path::{Path, PathBuf};

use config::redirect::{RedirectManifest, REDIRECTS_FILE};

use rusoto_s3::{S3Client, S3, GetObjectRequest};
use futures_util::StreamExt;
use tokio_util::codec;

use crate::Result;

pub(crate) async fn diff_redirects<P: AsRef<Path>>(
    client: &S3Client,
    target: P,
    bucket: &str,
    prefix: Option<String>,
    sync_redirects: bool) -> Result<(RedirectManifest, PathBuf)> {

    let (local, manifest_file) = load_local_redirects(target).await?;

    // If we are syncing redirects just use an 
    // empty remote redirects manifest
    let mut remote = if sync_redirects {
        Default::default() 
    // Otherwise fetch a manifest from the remote bucket
    } else {
        load_bucket_redirects(client, bucket, prefix).await? 
    };

    // Local redirects take precedence
    remote.map_mut().extend(
        local.map()
        .into_iter()
        .map(|(k, v)| (k.clone(), v.clone())));

    Ok((remote, manifest_file))
}

async fn load_bucket_redirects(
    client: &S3Client,
    bucket: &str,
    prefix: Option<String>,
) -> Result<RedirectManifest> {
    let key = if let Some(prefix) = prefix {
        format!("{}/{}", prefix, REDIRECTS_FILE)
    } else {
        REDIRECTS_FILE.to_string()
    };

    // Load remote `redirects.json` file.
    let req = GetObjectRequest {
        bucket: bucket.to_string(), 
        key,
        ..Default::default()
    };

    if let Ok(mut res) = client.get_object(req).await {
        if let Some(body) = res.body.take() {
            //let reader = body.into_async_read(); 

            let content = codec::FramedRead::new(
                body.into_async_read(), codec::BytesCodec::new())
                .into_future();

            let (res, _) = content.await;

            if let Some(bytes_result) = res {
                let bytes = bytes_result?;
                let buf: Vec<u8> = bytes.to_vec();
                let manifest: RedirectManifest = serde_json::from_slice(&buf)?;
                return Ok(manifest)
            }

        }
    }

    Ok(Default::default())
}

async fn load_local_redirects<P: AsRef<Path>>(
    target: P,
) -> Result<(RedirectManifest, PathBuf)> {
    let redirects_manifest_file = target.as_ref().join(REDIRECTS_FILE);
    if redirects_manifest_file.exists() {
        let contents = fs::read_to_string(&redirects_manifest_file)?;
        let manifest: RedirectManifest = serde_json::from_str(&contents)?;
        Ok((manifest, redirects_manifest_file))
    } else {
        Ok((Default::default(), redirects_manifest_file))
    }
}
