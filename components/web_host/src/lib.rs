mod cloudfront;
mod error;
mod region_info;
mod s3;

type Result<T> = std::result::Result<T, error::Error>;

pub use cloudfront::{DistributionSettings, ViewerProtocolPolicy, new_client as new_cloudfront_client};
pub use error::Error;
pub use s3::{BucketSettings, new_client as new_s3_client};
