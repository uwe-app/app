use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::{fs, io};

use rusoto_core::Region;
use rusoto_s3::{DeleteObjectRequest, PutObjectRequest, S3Client, S3};

use log::{debug, error, info};

use crate::{s3_util::*, Error, Result};
use config::redirect::RedirectManifest;

use super::{redirects, report::FileBuilder};

#[derive(Debug)]
pub struct PublishRequest {
    pub profile_name: String,
    pub region: Region,
    pub bucket: String,
    pub prefix: Option<String>,
    pub keep_remote: bool,
    pub build_target: PathBuf,
    pub sync_redirects: bool,
    pub redirects_manifest: Option<RedirectManifest>,
}

impl PublishRequest {
    fn new_client(&self) -> Result<S3Client> {
        get_client(&self.profile_name, &self.region)
    }
}

pub async fn publish(mut request: PublishRequest) -> Result<()> {
    let (file_builder, diff) = prepare_diff(&mut request).await?;
    sync_content(request, file_builder, diff).await
}

async fn prepare_diff(
    request: &mut PublishRequest,
) -> Result<(FileBuilder, DiffReport)> {
    let delimiter = utils::terminal::delimiter();

    println!("{}", &delimiter);
    println!(" PUBLISH");
    println!("{}", &delimiter);

    info!("Bucket {}", &request.bucket);
    info!("Building local file list");

    // Create the list of local build files
    let mut file_builder =
        FileBuilder::new(request.build_target.clone(), request.prefix.clone());
    file_builder.walk()?;

    info!("Local objects {}", file_builder.keys.len());
    info!("Building remote file list");

    let client = request.new_client()?;

    let mut remote: HashSet<String> = HashSet::new();
    let mut etags: HashMap<String, String> = HashMap::new();
    list_bucket_all(
        &client,
        &request.bucket,
        &request.prefix,
        &mut remote,
        &mut etags,
    )
    .await?;

    info!("Remote objects {}", remote.len());

    let diff = diff(&file_builder, &remote, &etags)?;

    let (redirects_manifest, redirects_manifest_file) =
        redirects::diff_redirects(
            &client,
            &request.build_target,
            &request.bucket,
            request.prefix.clone(),
            request.sync_redirects,
        )
        .await?;

    // Must overwrite the redirects file with new content
    // after merging the local and remote redirect manifests
    // and before uploading any content.
    fs::write(
        &redirects_manifest_file,
        serde_json::to_vec(&redirects_manifest)?,
    )?;
    request.redirects_manifest = Some(redirects_manifest);

    Ok((file_builder, diff))
}

async fn sync_content(
    mut request: PublishRequest,
    builder: FileBuilder,
    diff: DiffReport,
) -> Result<()> {
    let delimiter = utils::terminal::delimiter();

    println!("{}", &delimiter);
    println!(" DELTA");
    println!("{}", &delimiter);
    info!("New {}", diff.upload.len());
    info!("Update {}", diff.changed.len());
    info!("Delete {}", diff.deleted.len());

    let mut errors: Vec<Error> = Vec::new();
    let mut uploaded: u64 = 0;
    let mut deleted: u64 = 0;
    let client = request.new_client()?;

    let push: HashSet<_> = diff.upload.union(&diff.changed).collect();
    for k in push {
        let local_path = builder.from_key(&k);
        let req = PutObjectRequest {
            bucket: request.bucket.clone(),
            key: k.clone(),
            ..Default::default()
        };

        /*
        info!("Upload {}", local_path.display());
        info!("    -> {}", &k);
        */

        if let Err(e) =
            put_object_with_progress(&client, req, &local_path).await
        {
            errors.push(e);
        } else {
            uploaded += 1;
        }
    }

    if !diff.deleted.is_empty() && !request.keep_remote {
        println!("{}", &delimiter);
        println!(" DELETIONS");
        println!("{}", &delimiter);

        for k in &diff.deleted {
            let req = DeleteObjectRequest {
                bucket: request.bucket.clone(),
                key: k.clone(),
                ..Default::default()
            };

            info!("Delete {}", &k);

            if let Err(e) = client.delete_object(req).await {
                errors.push(Error::from(e));
            } else {
                deleted += 1;
            }
        }
    }

    let mut redirects = 0usize;

    if let Some(redirects_manifest) = request.redirects_manifest.take() {
        println!("{}", &delimiter);
        println!(" REDIRECTS");
        println!("{}", &delimiter);

        for (k, v) in redirects_manifest.map() {
            // NOTE: must not start with a slash!
            let redirect_key = k.trim_start_matches("/");

            info!("Redirect {} -> {}", redirect_key, &v);

            if let Err(e) =
                put_redirect(&client, &request.bucket, redirect_key, v).await
            {
                errors.push(e);
            } else {
                redirects += 1;
            }
        }
    }

    println!("{}", &delimiter);
    println!(" SUMMARY");
    println!("{}", &delimiter);

    info!("Uploads {}", uploaded);
    info!("Deleted {}", deleted);
    info!("Redirects {}", redirects);

    if !errors.is_empty() {
        println!("{}", &delimiter);
        println!(" ERRORS");
        println!("{}", &delimiter);

        for e in &errors {
            error!("{}", e);
        }
        error!("Errors {}", errors.len());
    }

    Ok(())
}

#[derive(Debug)]
struct DiffReport {
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

fn diff(
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
            let local_etag = read_file_etag(&local_path)?;
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
