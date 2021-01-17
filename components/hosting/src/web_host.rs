use serde::{Serialize, Deserialize};

use rusoto_core::{Region, RusotoError};
use rusoto_s3::{
    S3,
    S3Client,
    HeadBucketRequest,
    HeadBucketError,
    CreateBucketRequest,
    CreateBucketConfiguration,
};

use crate::{Error, Result};

static INDEX_HTML: &str = "index.html";
static ERROR_HTML: &str = "404.html";

#[derive(Debug, Serialize, Deserialize)]
pub struct WebHost {
    /// The region used for creating resources.
    region: Region,
    /// Name of the bucket
    bucket: String,
    /// Index page for static web hosting
    index_page: Option<String>,
    /// Error page for static web hosting
    error_page: Option<String>,
}

impl WebHost {
    pub fn new(region: Region, bucket: String) -> Self {
        Self {
            region,
            bucket,
            index_page: Some(INDEX_HTML.to_string()),
            error_page: Some(ERROR_HTML.to_string()),
        }
    }

    /// Ensure the bucket for this web host exists.
    pub async fn ensure_bucket(&self, client: &S3Client) -> Result<()> {
        let head_req =  HeadBucketRequest {
            bucket: self.bucket.to_string(),
            ..Default::default()
        };

        match client.head_bucket(head_req).await {
            Err(e) => {
                if let RusotoError::Service(ref service_error) = e {
                    if let HeadBucketError::NoSuchBucket(_) = service_error {
                        self.create_bucket(client).await
                    } else {
                        Err(Error::from(e))
                    }
                } else {
                    Err(Error::from(e))
                }
            }
            _ => Ok(())
        }
    }

    /// Create a bucket.
    pub async fn create_bucket(&self, client: &S3Client) -> Result<()> {
        let create_bucket_configuration = CreateBucketConfiguration {
            location_constraint: Some(self.region.name().to_string())
        };

        let create_req = CreateBucketRequest {
            bucket: self.bucket.to_string(),
            create_bucket_configuration: Some(create_bucket_configuration),
            ..Default::default()
        };
        client.create_bucket(create_req).await?;
        Ok(())
    }
}
