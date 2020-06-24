use std::path::PathBuf;

use core::future::Future;

use rusoto_core::request::HttpClient;
use rusoto_core::credential;
use rusoto_core::Region;
use rusoto_s3::*;

use crate::AwsResult;
use crate::Config;

#[derive(Debug)]
pub struct PublishRequest {
    pub profile_name: String,
    pub region: Region,
    pub bucket: String, 
}

#[derive(Debug)]
pub enum PublishProvider {
    Aws,
}

pub async fn publish(options: PublishRequest) -> AwsResult<()> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(options.profile_name);

    let dispatcher = HttpClient::new()?;
    let client = S3Client::new_with(dispatcher, provider, options.region);

    let req = HeadBucketRequest {
        bucket: options.bucket
    };

    let result = client.head_bucket(req).await?;
    println!("Publisher result {:?}", result);

    Ok(())
}
