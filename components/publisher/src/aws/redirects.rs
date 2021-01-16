use std::fs;
use std::path::{Path, PathBuf};

use config::redirect::{RedirectManifest, REDIRECTS_FILE};

use rusoto_s3::S3Client;

use crate::Result;

pub(crate) async fn diff_redirects<P: AsRef<Path>>(
    client: &S3Client,
    target: P,
    bucket: &str,
    prefix: Option<String>,
    prune_remote: bool) -> Result<(RedirectManifest, PathBuf)> {

    let (local, manifest_file) = load_local_redirects(target).await?;

    // If we are pruning remote redirects then we 
    // just use an empty remote redirects manifest
    let mut remote = if prune_remote {
        Default::default() 
    // Otherwise fetch from the remote bucket
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
    let mut out = Default::default();
    let key = if let Some(prefix) = prefix {
        format!("{}/{}", prefix, REDIRECTS_FILE)
    } else {
        REDIRECTS_FILE.to_string()
    };

    println!("Load remote redirects... {}", &key);

    // TODO: load remote `redirects.json` file.

    Ok(out)
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
