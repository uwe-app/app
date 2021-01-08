use std::collections::{HashMap, HashSet};
use std::io;
use std::io::Read;
use std::path::Path;

use std::str::FromStr;

use md5::{Digest, Md5};

use futures_util::TryStreamExt;
use tokio_util::codec;

use rusoto_core::credential;
use rusoto_core::request::HttpClient;
use rusoto_core::ByteStream;
use rusoto_core::Region;
use rusoto_s3::*;

use log::{debug, error, info};

use crate::{report::FileBuilder, Error, Result};

// The folder delimiter
static DELIMITER: &str = "/";

// Compute a digest from the file on disc and return it in
// an etag format.
fn get_file_etag<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let chunk_size = 16_000;
    let mut hasher = Md5::new();
    loop {
        let mut chunk = Vec::with_capacity(chunk_size);
        let n = file
            .by_ref()
            .take(chunk_size as u64)
            .read_to_end(&mut chunk)?;
        hasher.update(chunk);
        if n == 0 || n < chunk_size {
            break;
        }
    }
    Ok(format!("\"{:x}\"", hasher.finalize()))
}

pub fn parse_region<S: AsRef<str>>(s: S) -> Result<Region> {
    Ok(Region::from_str(s.as_ref())?)
}

#[derive(Debug)]
pub struct DiffReport {
    // Files that have the same checksums (etag)
    pub same: HashSet<String>,
    // New files that should be uploaded
    pub upload: HashSet<String>,
    // Files that exist in remote but have changed or new files
    // that should be uploaded
    pub changed: HashSet<String>,
    // Files that exist on remote but no longer exist locally
    pub deleted: HashSet<String>,
}

#[derive(Debug)]
pub struct PublishRequest {
    pub profile_name: String,
    pub region: Region,
    pub bucket: String,
    pub prefix: Option<String>,
    pub keep_remote: bool,
}

#[derive(Debug)]
pub enum PublishProvider {
    Aws,
}

fn get_client(profile: &str, region: &Region) -> Result<S3Client> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    let dispatcher = HttpClient::new()?;
    let client = S3Client::new_with(dispatcher, provider, region.clone());
    Ok(client)
}

fn get_client_request(request: &PublishRequest) -> Result<S3Client> {
    get_client(&request.profile_name, &request.region)
}

pub fn diff(
    builder: &FileBuilder,
    remote: &HashSet<String>,
    etags: &HashMap<String, String>,
) -> io::Result<DiffReport> {
    let local = &builder.keys;

    let mut same = HashSet::new();
    let mut upload = HashSet::new();
    let mut changed = HashSet::new();
    let mut deleted = HashSet::new();

    for k in local.intersection(&remote) {
        if let Some(etag) = etags.get(k) {
            let local_path = builder.from_key(&k);
            let local_etag = get_file_etag(&local_path)?;
            if etag == &local_etag {
                debug!("Checksum match {} {}", etag, local_path.display());
                same.insert(k.clone());
                continue;
            }
        }

        changed.insert(k.clone());
    }

    for k in local.difference(&remote) {
        upload.insert(k.clone());
    }

    for k in remote.difference(&local) {
        deleted.insert(k.clone());
    }

    Ok(DiffReport {
        same,
        upload,
        changed,
        deleted,
    })
}

async fn list_bucket_remote(
    client: &S3Client,
    request: &PublishRequest,
    continuation_token: Option<String>,
) -> Result<ListObjectsV2Output> {
    debug!("List bucket token {:?}", continuation_token);

    let req = ListObjectsV2Request {
        bucket: request.bucket.clone(),
        prefix: request.prefix.clone(),
        continuation_token,
        ..Default::default()
    };

    Ok(client.list_objects_v2(req).await?)
}

async fn fetch_bucket_remote(
    client: &S3Client,
    request: &PublishRequest,
    remote: &mut HashSet<String>,
    etags: &mut HashMap<String, String>,
) -> Result<()> {
    let mut continuation_token = None;
    loop {
        let result =
            list_bucket_remote(client, request, continuation_token).await?;
        if let Some(contents) = result.contents {
            debug!("List bucket contents length {}", contents.len());
            for obj in contents {
                if let Some(key) = obj.key {
                    if let Some(etag) = obj.e_tag {
                        etags.insert(key.clone(), etag);
                    }
                    // Do not include folder objects
                    if !key.ends_with(DELIMITER) {
                        remote.insert(key);
                    }
                }
            }
        }
        let is_truncated =
            result.is_truncated.is_some() && result.is_truncated.unwrap();
        if !is_truncated {
            break;
        } else {
            continuation_token = result.next_continuation_token;
        }
    }
    Ok(())
}

pub async fn list_remote(
    request: &PublishRequest,
    remote: &mut HashSet<String>,
    etags: &mut HashMap<String, String>,
) -> Result<()> {
    let client = get_client_request(request)?;
    fetch_bucket_remote(&client, &request, remote, etags).await?;
    Ok(())
}

pub async fn put_object<P: AsRef<Path>>(
    client: &S3Client,
    mut req: PutObjectRequest,
    path: P,
) -> Result<PutObjectOutput> {
    let file = std::fs::File::open(&path)?;
    let size = file.metadata()?.len();

    let tokio_file = tokio::fs::File::from_std(file);
    let stream = codec::FramedRead::new(tokio_file, codec::BytesCodec::new())
        .map_ok(|r| r.freeze());

    let body = ByteStream::new_with_size(stream, size as usize);
    req.body = Some(body);
    req.content_type = Some(
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string(),
    );

    Ok(client.put_object(req).await?)
}

/// Upload a single file creating a client for the request.
///
/// Use this for a single file upload; for multiple files
/// create  a client and call `put_object()`.
pub async fn put_file<F: AsRef<Path>>(
    file: F,
    key: &str,
    region: Region,
    bucket: &str,
    profile: &str,
) -> Result<PutObjectOutput> {
    let req = PutObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        ..Default::default()
    };

    let client = get_client(profile, &region)?;
    put_object(&client, req, file.as_ref()).await
}

/// Put a website redirect location.
pub async fn put_redirect<S: AsRef<str>>(
    location: S,
    key: &str,
    region: Region,
    bucket: &str,
    profile: &str,
) -> Result<PutObjectOutput> {
    let req = PutObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        website_redirect_location: Some(location.as_ref().to_string()),
        ..Default::default()
    };

    let client = get_client(profile, &region)?;
    Ok(client.put_object(req).await?)
}

async fn delete_object(
    client: &S3Client,
    req: DeleteObjectRequest,
) -> Result<DeleteObjectOutput> {
    Ok(client.delete_object(req).await?)
}

pub async fn publish(
    request: &PublishRequest,
    builder: FileBuilder,
    diff: DiffReport,
) -> Result<()> {
    if diff.upload.is_empty()
        && diff.changed.is_empty()
        && diff.deleted.is_empty()
    {
        info!("Site is up to date!");
        return Ok(());
    }

    let delimiter = "-".repeat(20);

    info!("{}", delimiter);
    info!("New {}", diff.upload.len());
    info!("Update {}", diff.changed.len());
    info!("Delete {}", diff.deleted.len());
    info!("{}", delimiter);

    let mut errors: Vec<Error> = Vec::new();
    let mut uploaded: u64 = 0;
    let mut deleted: u64 = 0;
    let client = get_client_request(request)?;

    let push: HashSet<_> = diff.upload.union(&diff.changed).collect();
    for k in push {
        let local_path = builder.from_key(&k);
        let req = PutObjectRequest {
            bucket: request.bucket.clone(),
            key: k.clone(),
            ..Default::default()
        };

        info!("Upload {}", local_path.display());
        info!("    -> {}", &k);

        if let Err(e) = put_object(&client, req, &local_path).await {
            errors.push(e);
        } else {
            uploaded += 1;
        }
    }

    if !request.keep_remote {
        for k in &diff.deleted {
            let req = DeleteObjectRequest {
                bucket: request.bucket.clone(),
                key: k.clone(),
                ..Default::default()
            };

            info!("Delete {}", &k);

            if let Err(e) = delete_object(&client, req).await {
                errors.push(e);
            } else {
                deleted += 1;
            }
        }
    }

    //info!("Ok (up to date) {}", diff.same.len());

    info!("{}", delimiter);
    info!("Uploads {}", uploaded);
    info!("Deleted {}", deleted);
    info!("{}", delimiter);

    if !errors.is_empty() {
        for e in &errors {
            error!("{}", e);
        }
        error!("Errors {}", errors.len());
    }

    Ok(())
}
