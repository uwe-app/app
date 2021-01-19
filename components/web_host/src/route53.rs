use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use log::{info, debug};
//use url::Url;

use rusoto_route53::{
    Route53,
    Route53Client,
};
use rusoto_core::{credential, request::HttpClient, Region};

use crate::{Error, Result};

pub fn new_client(profile: &str, region: &Region) -> Result<Route53Client> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    Ok(Route53Client::new_with(HttpClient::new()?, provider, region.clone()))
}
