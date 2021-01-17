use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
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
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),
}
