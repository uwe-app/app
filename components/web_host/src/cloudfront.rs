use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use log::{info, debug};
use url::Url;

use rusoto_cloudfront::{
    CloudFront,
    CloudFrontClient,
    ListDistributionsRequest,
    ListDistributionsResult,
    DistributionSummary,
    DistributionConfig,
    CreateDistributionRequest,
    UpdateDistributionRequest,
    Origin, Origins, DefaultCacheBehavior,
    ViewerCertificate, Aliases, CustomOriginConfig,
};
use rusoto_core::{credential, request::HttpClient, Region, RusotoError};

use crate::{Error, Result};

static MAX_ITEMS: usize = 100;

/*
allowed_methods  = ["GET", "HEAD"]
cached_methods   = ["GET", "HEAD"]

forwarded_values {
      query_string = false

      cookies {
        forward = "none"
      }
    }

min_ttl                = 0
default_ttl            = 86400
max_ttl                = 31536000
*/

#[derive(Debug, Serialize, Deserialize)]
pub enum ViewerProtocolPolicy {
    AllowAll,
    RedirectToHttps,
    HttpsOnly,
}

impl fmt::Display for ViewerProtocolPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::AllowAll => "allow-all",
            Self::RedirectToHttps => "redirect-to-https",
            Self::HttpsOnly => "https-only",
        })
    }
}

impl FromStr for ViewerProtocolPolicy {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "allow-all" => Ok(Self::AllowAll),
            "redirect-to-https" => Ok(Self::RedirectToHttps),
            "https-only" => Ok(Self::HttpsOnly),
            _ => Err(Error::UnknownViewerProtocolPolicy(s.to_string()))
        }
    }
}

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

    #[serde(skip)]
    domain_name: String,

    /// Unique identifier for the origin
    origin_id: String,

    /// Whether the distribution is enabled
    enabled: bool,

    /// Whether to automatically compress objects
    compress: bool,

    /// List of CNAME aliases
    aliases: Vec<String>,

    /// Viewer protocol policy.
    #[serde_as(as = "DisplayFromStr")]
    viewer_protocol_policy: ViewerProtocolPolicy,

    /// Caller refererence for the request.
    caller_reference: String,

    /// Distribution comment.
    comment: String,

    /// ARN for an ACM certificate
    acm_certificate_arn: Option<String>,

    /// Origin path
    origin_path: Option<String>,
}

impl DistributionSettings {
    pub fn new(origin: Url, aliases: Vec<String>, origin_id: Option<String>) -> Self {
        let domain_name = origin.domain()
            .expect("Origin URL must have a valid domain name")
            .to_string();
        Self {
            origin,
            origin_id: origin_id.clone().unwrap_or(domain_name.clone()),
            caller_reference: origin_id.clone().unwrap_or(domain_name.clone()),
            domain_name,
            comment: String::new(),
            aliases,
            enabled: true,
            compress: true,
            origin_path: None,
            acm_certificate_arn: None,
            viewer_protocol_policy: ViewerProtocolPolicy::AllowAll,
        }
    }

    /// Create a distribution.
    pub async fn create(&self, client: &CloudFrontClient) -> Result<()> {
        info!("Searching for {}", self.origin);
        let distributions = self.list_distributions_all(client).await?;
        for summary in distributions {
            for origin in summary.origins.items.iter() {
                debug!("Test domain name {}", origin.domain_name);
                if origin.domain_name == self.domain_name {
                    // Found an existing match so treat as an update operation
                    return self.update(client, summary.id).await
                }
            }
        }

        info!("Creating distribution for {}", self.origin);

        let distribution_config = self.into_distribution_config();
        debug!("Create {:#?}", &distribution_config);
        let req = CreateDistributionRequest {
            distribution_config,
        };

        let res = client.create_distribution(req).await?;

        if let (Some(location), Some(distribution)) = (res.location, res.distribution) {
            debug!("Created distribution {}", location);
            info!("Distribution id {}", distribution.id);
            info!("Distribution domain name {}", distribution.domain_name);
            info!("Status {} ✓", distribution.status);
        }

        Ok(())
    }

    /// Update a distribution.
    pub async fn update(&self, client: &CloudFrontClient, id: String) -> Result<()> {
        info!("Updating {} ({})", &id, self.origin);

        let distribution_config = self.into_distribution_config();
        debug!("Update {:#?}", &distribution_config);
        let req = UpdateDistributionRequest {
            id,
            distribution_config,
            ..Default::default()
        };

        // TODO: update

        Ok(())
    }

    /// List all distributions.
    pub async fn list_distributions_all(&self, client: &CloudFrontClient) -> Result<Vec<DistributionSummary>> {
        let mut out = Vec::new();
        let mut marker: Option<String> = None;
        loop {
            let mut result = self.list_distributions(client, marker.clone()).await?;
            if let Some(ref mut list) = result.distribution_list.as_mut() {
                if let Some(items) = list.items.take() {
                    out.extend(items);
                }
                let is_truncated = list.is_truncated;
                if !is_truncated {
                    break; 
                } else {
                    println!("Setting new marker {:?}", &list.next_marker);
                    marker = list.next_marker.clone();
                }
            }
        }
        Ok(out)
    }

    /// List distributions until `MAX_ITEMS` is reached.
    pub async fn list_distributions(
        &self,
        client: &CloudFrontClient,
        marker: Option<String>) -> Result<ListDistributionsResult> {
        let req = ListDistributionsRequest {
            marker,
            max_items: Some(MAX_ITEMS.to_string()),
            ..Default::default()
        };
        let res = client.list_distributions(req).await?;
        Ok(res)
    }

    /// Set whether the distribution is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set whether to automatically compress objects.
    pub fn set_compress(&mut self, compress: bool) {
        self.compress = compress;
    }

    /// Set the viewer protocol policy.
    pub fn set_viewer_protocol_policy(&mut self, policy: ViewerProtocolPolicy) {
        self.viewer_protocol_policy = policy;
    }

    /// Set a comment for the distribution.
    pub fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }

    /// Set an ACM certificate ARN.
    pub fn set_acm_certificate_arn(&mut self, acm_certificate_arn: Option<String>) {
        self.acm_certificate_arn = acm_certificate_arn;
    }

    /// Set an origin path for the distribution.
    pub fn set_origin_path(&mut self, origin_path: Option<String>) {
        self.origin_path = origin_path;
    }

    fn into_viewer_certificate(&self) -> Option<ViewerCertificate> {
        Some(ViewerCertificate {
            acm_certificate_arn: self.acm_certificate_arn.clone(),
            ssl_support_method: Some("sni-only".to_string()),
            ..Default::default()
        })
    }

    fn into_aliases(&self) -> Option<Aliases> {
        if !self.aliases.is_empty() {
            Some(Aliases {
                quantity: self.aliases.len() as i64,
                items: Some(self.aliases.clone())
            })
        } else { None }
    }

    /// Convert into a distribution config suitable for 
    /// creating or updating a distribution.
    fn into_distribution_config(&self) -> DistributionConfig {

        let default_cache_behavior = DefaultCacheBehavior {
            compress: Some(self.compress.clone()),
            target_origin_id: self.origin_id.clone(),
            // SEE: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/using-managed-cache-policies.html
            cache_policy_id: Some("Managed-CachingOptimized".to_string()),
            viewer_protocol_policy: self.viewer_protocol_policy.to_string(),
            ..Default::default()
        };

        let aliases = self.into_aliases();

        let origin = Origin {
            domain_name: self.domain_name.clone(), 
            id: self.origin_id.clone(),
            origin_path: self.origin_path.clone(),

            custom_origin_config: Some(
                CustomOriginConfig {
                    http_port: 80,
                    https_port: 443,
                    origin_protocol_policy: "http-only".to_string(),
                    ..Default::default()
                }   
            ),
            /*
            // SEE: https://github.com/hashicorp/terraform/issues/6422
            s3_origin_config: Some(
                S3OriginConfig {
                    origin_access_identity: String::new(),
                }),
            */
            ..Default::default()
        };

        let origins = Origins {
            items: vec![origin],
            quantity: 1,
        };

        DistributionConfig {
            origins,
            aliases,
            caller_reference: self.caller_reference.clone(),
            comment: self.comment.clone(),
            enabled: self.enabled.clone(),
            default_cache_behavior,
            viewer_certificate: self.into_viewer_certificate(),
            is_ipv6_enabled: Some(true),
            ..Default::default()
        }
    }
}
