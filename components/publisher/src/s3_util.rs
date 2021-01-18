use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

use md5::{Digest, Md5};

use rusoto_core::credential;
use rusoto_core::request::HttpClient;
use rusoto_core::ByteStream;
use rusoto_core::Region;
use rusoto_s3::*;

use futures::TryStreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

use log::debug;
use pbr::{ProgressBar, Units};
use read_progress_stream::ReadProgressStream;

use crate::Result;

pub fn parse_region<S: AsRef<str>>(s: S) -> Result<Region> {
    Ok(Region::from_str(s.as_ref())?)
}

// Compute a digest from the file on disc and return it in
// an etag format.
pub fn read_file_etag<P: AsRef<Path>>(path: P) -> io::Result<String> {
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

pub fn get_client(profile: &str, region: &Region) -> Result<S3Client> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    let dispatcher = HttpClient::new()?;
    let client = S3Client::new_with(dispatcher, provider, region.clone());
    Ok(client)
}

pub async fn list_bucket(
    client: &S3Client,
    bucket: &str,
    prefix: &Option<String>,
    continuation_token: Option<String>,
) -> Result<ListObjectsV2Output> {
    debug!("List bucket token {:?}", continuation_token);

    let req = ListObjectsV2Request {
        bucket: bucket.to_string(),
        prefix: prefix.clone(),
        continuation_token,
        ..Default::default()
    };

    Ok(client.list_objects_v2(req).await?)
}

pub async fn list_bucket_all(
    client: &S3Client,
    bucket: &str,
    prefix: &Option<String>,
    remote: &mut HashSet<String>,
    etags: &mut HashMap<String, String>,
) -> Result<()> {
    let mut continuation_token = None;
    loop {
        let result =
            list_bucket(client, bucket, prefix, continuation_token).await?;
        if let Some(contents) = result.contents {
            debug!("List bucket contents length {}", contents.len());
            for obj in contents {
                if let Some(key) = obj.key {
                    if let Some(etag) = obj.e_tag {
                        etags.insert(key.clone(), etag);
                    }
                    // Do not include folder objects
                    if !key.ends_with("/") {
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

pub async fn put_object_with_progress<P: AsRef<Path>>(
    client: &S3Client,
    mut req: PutObjectRequest,
    path: P,
) -> Result<PutObjectOutput> {
    /*
    let file = std::fs::File::open(&path)?;
    let size = file.metadata()?.len();
    let file = tokio::fs::File::from_std(file);
    */

    let file = tokio::fs::File::open(&path).await?;
    let size = file.metadata().await?.len();
    let reader =
        FramedRead::new(file, BytesCodec::new()).map_ok(|r| r.freeze());

    let mut pb = ProgressBar::new(size);
    pb.set_units(Units::Bytes);
    pb.show_speed = false;

    if let Some(name) = path.as_ref().file_name() {
        let msg = format!(" Upload {} ", name.to_string_lossy());
        pb.message(&msg);
    }

    let progress = Box::new(move |amount: u64, _| {
        pb.add(amount);
    });

    let stream = ReadProgressStream::new(reader, progress);

    let body = ByteStream::new_with_size(stream, size as usize);
    req.body = Some(body);
    req.content_type = Some(
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string(),
    );

    Ok(client.put_object(req).await?)
}

/// Put a website redirect location.
pub async fn put_redirect<S: AsRef<str>>(
    client: &S3Client,
    bucket: &str,
    key: &str,
    location: S,
) -> Result<PutObjectOutput> {
    let req = PutObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        website_redirect_location: Some(location.as_ref().to_string()),
        ..Default::default()
    };

    Ok(client.put_object(req).await?)
}

/// Upload a single file creating a transient client for the request.
///
/// Use this for a single file upload; for multiple files
/// create a client and call `put_object_with_progress()`.
pub async fn put_object_file_once<F: AsRef<Path>>(
    profile: &str,
    region: &Region,
    bucket: &str,
    key: &str,
    file: F,
) -> Result<PutObjectOutput> {
    let req = PutObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        ..Default::default()
    };
    let client = get_client(profile, region)?;
    put_object_with_progress(&client, req, file.as_ref()).await
}

/// Put a website redirect location and create a transient
/// client to process the request.
pub async fn put_redirect_once<S: AsRef<str>>(
    profile: &str,
    region: &Region,
    bucket: &str,
    key: &str,
    location: S,
) -> Result<PutObjectOutput> {
    let client = get_client(profile, region)?;
    Ok(put_redirect(&client, bucket, key, location).await?)
}
