use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use log::info;
use url::Url;

use rusoto_cloudfront::{CloudFront, CloudFrontClient};
use rusoto_core::{credential, request::HttpClient, Region, RusotoError};

use crate::{Error, Result};

pub fn new_client(profile: &str, region: &Region) -> Result<CloudFrontClient> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    let dispatcher = HttpClient::new()?;
    let client =
        CloudFrontClient::new_with(dispatcher, provider, region.clone());
    Ok(client)
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct DistributionSettings {
    /// The origin URL for the distribution
    #[serde_as(as = "DisplayFromStr")]
    origin: Url,

    /// List of CNAME aliases
    alias: Vec<String>,
}

impl DistributionSettings {
    pub fn new(origin: Url, alias: Vec<String>) -> Self {
        Self { origin, alias }
    }

    pub async fn create(&self, client: &CloudFrontClient) -> Result<()> {
        println!("Create a cdn distribution {}", self.origin);
        println!("Create a cdn distribution {:?}", self.alias);
        Ok(())
    }
}
