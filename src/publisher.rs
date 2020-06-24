use rusoto_core::request::HttpClient;
use rusoto_core::credential;
use rusoto_core::Region;
use rusoto_s3::*;

use crate::AwsResult;

#[derive(Debug)]
pub struct PublishRequest {
    pub profile_name: String,
    pub region: Region,
    pub bucket: String, 
    pub path: String,
}

#[derive(Debug)]
pub enum PublishProvider {
    Aws,
}

pub async fn publish(request: PublishRequest) -> AwsResult<()> {

    println!("Publisher request {:?}", request);

    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(request.profile_name);

    let dispatcher = HttpClient::new()?;
    let client = S3Client::new_with(dispatcher, provider, request.region);

    let req = HeadBucketRequest {
        bucket: request.bucket
    };

    let result = client.head_bucket(req).await?;
    println!("Publisher result {:?}", result);

    Ok(())
}
