mod acm;
mod cloudfront;
mod dns_client;
mod error;
mod name_servers;
mod region_info;
mod route53;
mod s3;
mod state_machine;

type Result<T> = std::result::Result<T, error::Error>;

pub use acm::{new_client as new_acm_client, CertSettings, CertUpsert};
pub use cloudfront::{
    new_client as new_cloudfront_client, DistributionSettings,
    ViewerProtocolPolicy,
};
pub use error::Error;
pub use name_servers::list as list_name_servers;
pub use route53::{
    new_client as new_route53_client, DnsRecord, DnsSettings, HostedZoneUpsert,
    RecordType, ZoneSettings,
};
pub use s3::{new_client as new_s3_client, BucketSettings};
pub use state_machine::{
    ensure_domain, ensure_website, load_host_file, WebHostRequest,
    WebHostResponse,
};

pub use rusoto_route53;

pub fn trim_hosted_zone_id(id: &str) -> String {
    id.trim_start_matches("/hostedzone/").to_string()
}
