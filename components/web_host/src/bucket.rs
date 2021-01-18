use serde::{Deserialize, Serialize};

use log::info;

use rusoto_core::{Region, RusotoError};
use rusoto_s3::{
    CreateBucketConfiguration, CreateBucketRequest, ErrorDocument,
    HeadBucketError, HeadBucketRequest, IndexDocument, PutBucketPolicyRequest,
    PutBucketWebsiteRequest, S3Client, WebsiteConfiguration, S3,
    PutPublicAccessBlockRequest, PublicAccessBlockConfiguration,
};

use crate::{Error, Result, region_info::REGION_INFO};

//static INDEX_HTML: &str = "index.html";
//static ERROR_HTML: &str = "404.html";

static BUCKET_TEMPLATE: &str = "__BUCKET__";
static POLICY_TEMPLATE: &str = include_str!("bucket_policy.json");

#[derive(Debug, Serialize, Deserialize)]
pub struct BucketHost {
    /// The region used for creating resources.
    region: Region,
    /// Name of the bucket
    bucket: String,
    /// Index page for static web hosting
    index_page: Option<String>,
    /// Error page for static web hosting
    error_page: Option<String>,

    #[serde(skip)]
    policy: Option<String>,
}

impl BucketHost {
    pub fn new(
        region: Region,
        bucket: String,
        index: String,
        error: String) -> Self {
        let policy = POLICY_TEMPLATE.replace(BUCKET_TEMPLATE, &bucket);
        Self {
            region,
            bucket,
            index_page: Some(index),
            error_page: Some(error),
            policy: Some(policy),
        }
    }

    /// Bring this web host up.
    pub async fn up(&self, client: &S3Client) -> Result<()> {
        info!("Ensure bucket {}", &self.bucket);
        self.ensure_bucket(client).await?;
        info!("Disable public access block {}", &self.bucket);
        self.put_public_access_block(client).await?;
        info!("Set bucket policy {}", &self.bucket);
        self.put_bucket_policy(client).await?;
        info!("Set static website hosting {}", &self.bucket);
        self.put_bucket_website(client).await?;

        info!("{} ✓", self.url());

        Ok(())
    }

    /// Get the endpoint for the website.
    pub fn endpoint(&self) -> String {
        let region_info = REGION_INFO.get(&self.region)
            .expect("Unable to find region info for a region!");
        format!("{}.{}", &self.bucket, &region_info.s3_endpoint_suffix)
    }

    /// Get the endpoint URL for the website.
    pub fn url(&self) -> String {
        format!("http://{}", self.endpoint())
    }

    /// Ensure the bucket for this web host exists.
    async fn ensure_bucket(&self, client: &S3Client) -> Result<()> {
        let req = HeadBucketRequest {
            bucket: self.bucket.to_string(),
            ..Default::default()
        };

        match client.head_bucket(req).await {
            Err(e) => {
                // NOTE: The docs have the `NoSuchBucket` enum variant 
                // NOTE: but it actually triggers as a raw response 
                // NOTE: with a `404` status. We check for both in case 
                // NOTE: this changes in the future.
                if let RusotoError::Unknown(ref http_res) = e {
                    if http_res.status == 404 {
                        self.create_bucket(client).await
                    } else {
                        Err(Error::from(e))
                    }
                } else if let RusotoError::Service(ref service_error) = e {
                    #[allow(irrefutable_let_patterns)]
                    if let HeadBucketError::NoSuchBucket(_) = service_error {
                        self.create_bucket(client).await
                    } else {
                        Err(Error::from(e))
                    }
                } else {
                    Err(Error::from(e))
                }
            }
            _ => Ok(()),
        }
    }

    /// Allow public access configuration.
    async fn put_public_access_block(&self, client: &S3Client) -> Result<()> {
        let public_access_block_configuration = PublicAccessBlockConfiguration {
            block_public_acls: Some(false),
            block_public_policy: Some(false),
            ignore_public_acls: Some(false),
            restrict_public_buckets: Some(false),
        };

        let req = PutPublicAccessBlockRequest {
            bucket: self.bucket.to_string(),
            public_access_block_configuration,
            ..Default::default()
        };
        client.put_public_access_block(req).await?;
        Ok(())
    }

    /// Create a bucket.
    async fn create_bucket(&self, client: &S3Client) -> Result<()> {
        let create_bucket_configuration = CreateBucketConfiguration {
            location_constraint: Some(self.region.name().to_string()),
        };

        let req = CreateBucketRequest {
            bucket: self.bucket.to_string(),
            create_bucket_configuration: Some(create_bucket_configuration),
            ..Default::default()
        };
        client.create_bucket(req).await?;
        Ok(())
    }

    /// Set the bucket policy to allow public reads.
    async fn put_bucket_policy(&self, client: &S3Client) -> Result<()> {
        let req = PutBucketPolicyRequest {
            bucket: self.bucket.to_string(),
            policy: self.policy.clone().unwrap(),
            ..Default::default()
        };
        client.put_bucket_policy(req).await?;
        Ok(())
    }

    /// Set the bucket policy to allow public reads.
    async fn put_bucket_website(&self, client: &S3Client) -> Result<()> {
        let website_configuration = WebsiteConfiguration {
            index_document: self
                .index_page
                .clone()
                .map(|s| IndexDocument { suffix: s.clone() }),
            error_document: self
                .error_page
                .clone()
                .map(|s| ErrorDocument { key: s.clone() }),
            ..Default::default()
        };
        let req = PutBucketWebsiteRequest {
            bucket: self.bucket.to_string(),
            website_configuration,
            ..Default::default()
        };
        client.put_bucket_website(req).await?;
        Ok(())
    }
}
