use std::path::PathBuf;

use rusoto_core::request::HttpClient;
use rusoto_core::credential;
use rusoto_core::Region;
use rusoto_s3::*;

use crate::Error;
use crate::Result;
use crate::config::{Config, AwsPublishConfig};

use crate::publisher::{self, PublishRequest, PublishProvider};

#[derive(Debug)]
pub struct PublishOptions {
    pub project: PathBuf,
    pub profile: String,
    pub provider: PublishProvider,
}

fn find_aws_config<'a>(config: &'a Config, name: &str) -> Option<&'a AwsPublishConfig> {
    if let Some(ref publish) = config.publish {
        return publish.aws.get(name);
    }
    None
}

#[tokio::main]
pub async fn publish(options: PublishOptions) -> Result<()> {
    let config = Config::load(&options.project, false)?;

    match options.provider {
        PublishProvider::Aws => {

            if let Some(publish_config) = find_aws_config(&config, &options.profile) {
                let request = PublishRequest {
                    profile_name: publish_config.credentials.clone(),
                    // TODO: get a region
                    region: Region::ApSoutheast1,
                    bucket: publish_config.bucket.as_ref().unwrap().clone(),
                };

                println!("Trying to publish");

                publisher::publish(request).await;
            } else {
                return Err(Error::new(format!("Unknown publish profile {}", &options.profile)))
            }

        },
    }

    //let mut provider = credential::ProfileProvider::new()?;
    //provider.set_profile("tmpfs".to_string());

    //let dispatcher = HttpClient::new()?;

    ////let region = Region::default();
    //let region = Region::ApSoutheast1;
    //let client = S3Client::new_with(dispatcher, provider, region);

    //let req = HeadBucketRequest {
        //bucket: "tmpfs.org".to_string()
    //};

    //let result = client.head_bucket(req).await?;


    //result.foo();

    Ok(())
}
