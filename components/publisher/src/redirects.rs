use std::fs;
use std::path::Path;

use config::redirect::{RedirectManifest, REDIRECTS_FILE};

use rusoto_s3::S3Client;

use crate::Result;

pub(crate) fn load_bucket_redirects(
    client: &S3Client,
    bucket: String,
    prefix: Option<String>,
) -> Result<RedirectManifest> {
    let mut out = Default::default();
    let key = if let Some(prefix) = prefix {
        format!("{}/{}", prefix, REDIRECTS_FILE)
    } else {
        REDIRECTS_FILE.to_string()
    };

    // TODO: load remote `redirects.json` file.

    Ok(out)
}

pub(crate) fn load_local_redirects<P: AsRef<Path>>(
    target: P,
) -> Result<RedirectManifest> {
    let redirects_manifest_file = target.as_ref().join(REDIRECTS_FILE);
    if redirects_manifest_file.exists() {
        let contents = fs::read_to_string(&redirects_manifest_file)?;
        let manifest: RedirectManifest = serde_json::from_str(&contents)?;
        Ok(manifest)
    } else {
        Ok(Default::default())
    }
}
