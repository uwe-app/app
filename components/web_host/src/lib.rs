mod cloudfront;
mod error;
mod region_info;
mod route53;
mod s3;

type Result<T> = std::result::Result<T, error::Error>;

pub use cloudfront::{
    new_client as new_cloudfront_client, DistributionSettings,
    ViewerProtocolPolicy,
};
pub use error::Error;
pub use route53::{
    new_client as new_route53_client, DnsRecord, DnsSettings, RecordType,
    ZoneSettings,
};
pub use s3::{new_client as new_s3_client, BucketSettings};
