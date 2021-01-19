use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown viewer protocol policy {0}")]
    UnknownViewerProtocolPolicy(String),

    #[error("Unable to find a cache policy matching the name {0}")]
    NoCachePolicy(String),

    #[error(transparent)]
    Tls(#[from] rusoto_core::request::TlsError),

    #[error(transparent)]
    Credentials(#[from] rusoto_core::credential::CredentialsError),

    #[error(transparent)]
    HeadBucket(#[from] rusoto_core::RusotoError<rusoto_s3::HeadBucketError>),

    #[error(transparent)]
    CreateBucket(
        #[from] rusoto_core::RusotoError<rusoto_s3::CreateBucketError>,
    ),

    #[error(transparent)]
    PutBucketPolicy(
        #[from] rusoto_core::RusotoError<rusoto_s3::PutBucketPolicyError>,
    ),

    #[error(transparent)]
    PutBucketWebsite(
        #[from] rusoto_core::RusotoError<rusoto_s3::PutBucketWebsiteError>,
    ),

    #[error(transparent)]
    PutPublicAccessBlock(
        #[from] rusoto_core::RusotoError<rusoto_s3::PutPublicAccessBlockError>,
    ),

    #[error(transparent)]
    ListDistributions(
        #[from] rusoto_core::RusotoError<rusoto_cloudfront::ListDistributionsError>,
    ),

    #[error(transparent)]
    CreateDistribution(
        #[from] rusoto_core::RusotoError<rusoto_cloudfront::CreateDistributionError>,
    ),

    #[error(transparent)]
    UpdateDistribution(
        #[from] rusoto_core::RusotoError<rusoto_cloudfront::UpdateDistributionError>,
    ),

    #[error(transparent)]
    ListCachePolicies(
        #[from] rusoto_core::RusotoError<rusoto_cloudfront::ListCachePoliciesError>,
    ),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),
}
