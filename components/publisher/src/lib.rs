use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Tls(#[from] rusoto_core::request::TlsError),
    #[error(transparent)]
    ParseRegion(#[from] rusoto_signature::region::ParseRegionError),
    #[error(transparent)]
    Credentials(#[from] rusoto_core::credential::CredentialsError),
    #[error(transparent)]
    GetObject(#[from] rusoto_core::RusotoError<rusoto_s3::GetObjectError>),
    #[error(transparent)]
    HeadBucket(#[from] rusoto_core::RusotoError<rusoto_s3::HeadBucketError>),
    #[error(transparent)]
    PutObject(#[from] rusoto_core::RusotoError<rusoto_s3::PutObjectError>),
    #[error(transparent)]
    DeleteObject(
        #[from] rusoto_core::RusotoError<rusoto_s3::DeleteObjectError>,
    ),
    #[error(transparent)]
    ListObjects(
        #[from] rusoto_core::RusotoError<rusoto_s3::ListObjectsV2Error>,
    ),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum PublishProvider {
    Aws,
}

mod aws;
mod s3_util;

pub use aws::provider::{
    publish as aws_publish,
    PublishRequest as AwsPublishRequest
};

pub use s3_util::*;
