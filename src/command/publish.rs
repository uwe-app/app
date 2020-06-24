use rusoto_core::request::HttpClient;
use rusoto_core::credential;
use rusoto_core::Region;
use rusoto_s3::*;

use crate::AwsResult;

#[derive(Debug)]
pub enum PublishProvider {
    Aws,
}

#[derive(Debug)]
pub struct PublishOptions {
    pub profile: String,
    pub provider: PublishProvider,
}

#[tokio::main]
pub async fn publish(options: PublishOptions) -> AwsResult<()> {

    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile("tmpfs".to_string());

    let dispatcher = HttpClient::new()?;

    //let region = Region::default();
    let region = Region::ApSoutheast1;
    let client = S3Client::new_with(dispatcher, provider, region);

    let req = HeadBucketRequest {
        bucket: "tmpfs.org".to_string()
    };

    let result = client.head_bucket(req).await?;


    //result.foo();

    Ok(())
}
