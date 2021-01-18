mod bucket;
mod region_info;
mod error;

type Result<T> = std::result::Result<T, error::Error>;

use rusoto_core::credential;
use rusoto_core::request::HttpClient;
use rusoto_core::Region;
use rusoto_s3::S3Client;

pub fn new_client(profile: &str, region: &Region) -> Result<S3Client> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    let dispatcher = HttpClient::new()?;
    let client = S3Client::new_with(dispatcher, provider, region.clone());
    Ok(client)
}

pub use error::Error;
pub use bucket::BucketHost;
