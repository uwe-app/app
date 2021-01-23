use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use async_trait::async_trait;
use log::debug;
use url::Url;
use rusoto_core::Region;
use slug::slugify;

use crate::{
    to_idna_punycode,
    name_servers, new_acm_client, new_cloudfront_client, new_route53_client,
    new_s3_client, trim_hosted_zone_id, BucketSettings, CertSettings,
    CertUpsert, Error, HostedZoneUpsert, Result, ZoneSettings, DistributionSettings,
    DistributionUpsert, ViewerProtocolPolicy, DnsRecord, RecordType, DnsSettings,
};

static WWW: &str = "www";
static INDEX_SUFFIX: &str = "index.html";
static ERROR_KEY: &str = "404.html";

/// Asynchronous fallible transition from a state
/// to the next state.
#[async_trait]
pub trait Transition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>>;
}

/// Enumeration of available states.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum State {
    NameServer,
    HostedZone,
    Certificate,
    Bucket,
    RedirectBucket,
    Cdn,
    CdnDns,
    RedirectCname,
}

/// State machine iterates available states and yields a
/// transition for each state.
///
/// Iterators should invoke the `next()` function on the yielded
/// transition to get the next state and then call `advance()` on
/// the state machine to jump to the next state.
///
/// The next state must exist in the list of iterable states
/// otherwise iteration is halted.
#[derive(Debug)]
pub struct StateMachine<'a> {
    states: &'a [State],
    index: usize,
}

impl<'a> StateMachine<'a> {
    pub fn new(states: &'a [State]) -> Self {
        Self { states, index: 0 }
    }

    /// Advance the index to the next state
    /// returned by a transition function.
    fn advance(&mut self, state: State) {
        let index = self.states.iter().position(|r| r == &state);
        if let Some(index) = index {
            self.index = index;
        } else {
            // Nowhere to go so prevent any more iteration.
            self.stop();
        }
    }

    /// Stop iteration by moving the state index out of bounds.
    fn stop(&mut self) {
        self.index = self.states.len()
    }
}

/// Iterator yields a transition for a state.
impl<'a> Iterator for StateMachine<'a> {
    type Item = (State, Box<dyn Transition>);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(state) = self.states.get(self.index) {
            let item = match state {
                State::NameServer => {
                    let transition: Box<dyn Transition> =
                        Box::new(NameServerTransition {});
                    Some((state.clone(), transition))
                }
                State::HostedZone => {
                    let transition: Box<dyn Transition> =
                        Box::new(HostedZoneTransition {});
                    Some((state.clone(), transition))
                }
                State::Certificate => {
                    let transition: Box<dyn Transition> =
                        Box::new(CertificateTransition {});
                    Some((state.clone(), transition))
                }
                State::Bucket => {
                    let transition: Box<dyn Transition> =
                        Box::new(BucketTransition {});
                    Some((state.clone(), transition))
                }
                State::RedirectBucket => {
                    let transition: Box<dyn Transition> =
                        Box::new(RedirectBucketTransition {});
                    Some((state.clone(), transition))
                }
                State::Cdn => {
                    let transition: Box<dyn Transition> =
                        Box::new(CdnTransition {});
                    Some((state.clone(), transition))
                }
                State::CdnDns => {
                    let transition: Box<dyn Transition> =
                        Box::new(CdnDnsTransition {});
                    Some((state.clone(), transition))
                }
                State::RedirectCname => {
                    let transition: Box<dyn Transition> =
                        Box::new(RedirectCnameTransition {});
                    Some((state.clone(), transition))
                }
            };
            item
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WebHostRequest {
    /// Name of the AWS credentials profile to use.
    credentials: String,
    /// Domain name to host.
    domain_name: String,
    /// Region for resources, currently only S3 buckets
    /// require region selection.
    region: Region,
    /// Alternative domain names for the SSL certificate.
    subject_alternative_names: Option<Vec<String>>,
    /// Name for the S3 bucket.
    bucket_name: String,
    /// The suffix for folder requests to a bucket.
    index_suffix: String,
    /// The key for a bucket error document.
    error_key: String,
    /// Domain name for the redirecgt bucket.
    ///
    /// If no name is given `www` is prefixed to the default domain name,
    redirect_domain_name: Option<String>,
    /// Redirect all requests from this bucket to the primary
    /// bucket. Useful for configuring `www` to redirect to the
    /// primary domain.
    redirect_bucket_name: Option<String>,
    /// Protocol used for redirects.
    ///
    /// Recommended to leave this empty so that the redirect
    /// uses the protocol for the request but could be used
    /// to force `https` redirects if required.
    redirect_protocol: Option<String>,

    /// Monitor for certificate status
    monitor_certificate: bool,
    /// Timeout when monitoring certificate status.
    monitor_certificate_timeout: u64,
}

impl WebHostRequest {
    /// If we are just checking the domamin name servers no
    /// AWS calls are required.
    pub fn new_domain(domain_name: String) -> Self {
        Self {
            domain_name,
            ..Default::default()
        }
    }

    pub fn set_credentials(&mut self, credentials: String) {
        self.credentials = credentials;
    }
}

impl Default for WebHostRequest {
    fn default() -> Self {
        Self {
            credentials: String::new(),
            region: Region::UsEast1,
            bucket_name: String::new(),
            index_suffix: INDEX_SUFFIX.to_string(),
            error_key: ERROR_KEY.to_string(),
            subject_alternative_names: None,
            redirect_domain_name: None,
            redirect_bucket_name: None,
            redirect_protocol: None,
            domain_name: String::new(),
            monitor_certificate: true,
            monitor_certificate_timeout: 300,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WebHostResponse {
    /// Whether the name servers have propagated.
    pub name_servers_propagated: bool,
    /// Hosted zone id.
    pub zone_id: String,
    /// An ARN identifier for the SSL certificate.
    pub certificate_arn: String,
    /// Endpoint for the primary bucket as a host name.
    pub bucket_endpoint: String,
    /// Endpoint for the redirect bucket as a host name.
    pub redirect_bucket_endpoint: Option<String>,
    /// Domain name for the CDN.
    pub cdn_domain_name: String,
    /// Distribution id.
    pub cdn_distribution_id: String,
}

struct NameServerTransition;

#[async_trait]
impl Transition for NameServerTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {
        let dns_domain_name = name_servers::qualified(&request.domain_name);
        let ns_result = name_servers::lookup(&dns_domain_name).await?;
        if ns_result.is_propagated() {
            response.name_servers_propagated = true;
            Ok(Some(State::HostedZone))
        } else {
            Err(Error::NameServerPropagation)
        }
    }
}

struct HostedZoneTransition;

#[async_trait]
impl Transition for HostedZoneTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {
        let client = new_route53_client(&request.credentials)?;
        let zone = ZoneSettings::new();
        match zone
            .upsert(&client, request.domain_name.to_string())
            .await?
        {
            HostedZoneUpsert::Create(res) => {
                response.zone_id = trim_hosted_zone_id(&res.hosted_zone.id);
            }
            HostedZoneUpsert::Exists(res) => {
                response.zone_id = trim_hosted_zone_id(&res.id);
            }
        }

        Ok(Some(State::Certificate))
    }
}

struct CertificateTransition;

#[async_trait]
impl Transition for CertificateTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {
        let client = new_acm_client(&request.credentials)?;
        let dns_client = new_route53_client(&request.credentials)?;
        let cert = CertSettings::new();
        match cert
            .upsert(
                &client,
                &dns_client,
                request.domain_name.to_string(),
                request.subject_alternative_names.clone(),
                response.zone_id.clone(),
                request.monitor_certificate,
                request.monitor_certificate_timeout,
            )
            .await?
        {
            CertUpsert::Create(arn) => {
                response.certificate_arn = arn;
            }
            CertUpsert::Exists(details) => {
                response.certificate_arn = details.certificate_arn.unwrap();
            }
        }
        Ok(Some(State::Bucket))
    }
}

struct BucketTransition;

#[async_trait]
impl Transition for BucketTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {
        let client = new_s3_client(&request.credentials, &request.region)?;
        let bucket = BucketSettings::new(
            request.region.clone(),
            request.bucket_name.clone(),
            request.index_suffix.clone(),
            request.error_key.clone(),
            None,
            None,
        );

        response.bucket_endpoint = bucket.up(&client).await?;

        if let Some(_) = request.redirect_bucket_name {
            Ok(Some(State::RedirectBucket))
        } else {
            Ok(Some(State::Cdn))
        }
    }
}

struct RedirectBucketTransition;

#[async_trait]
impl Transition for RedirectBucketTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {
        let client = new_s3_client(&request.credentials, &request.region)?;

        let bucket = BucketSettings::new(
            request.region.clone(),
            request.redirect_bucket_name.clone().unwrap(),
            request.index_suffix.clone(),
            request.error_key.clone(),
            Some(request.domain_name.clone()),
            request.redirect_protocol.clone(),
        );

        response.redirect_bucket_endpoint = Some(bucket.up(&client).await?);
        Ok(Some(State::Cdn))
    }
}

struct CdnTransition;

#[async_trait]
impl Transition for CdnTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {
        let client =
            new_cloudfront_client(&request.credentials)?;

        let origin: Url = format!("http://{}", &response.bucket_endpoint).parse()?;
        let alias = vec![to_idna_punycode(&request.domain_name)?];
        let origin_id = slugify(&request.domain_name);

        let mut cdn =
            DistributionSettings::new(origin, alias, Some(origin_id));
        cdn.set_acm_certificate_arn(Some(response.certificate_arn.clone()));
        cdn.set_viewer_protocol_policy(ViewerProtocolPolicy::RedirectToHttps);

        match cdn.upsert(&client).await? {
            DistributionUpsert::Create(dist) => {
                response.cdn_domain_name = dist.domain_name;
                response.cdn_distribution_id = dist.id;
            }
            DistributionUpsert::Exists(dist) => {
                response.cdn_domain_name = dist.domain_name;
                response.cdn_distribution_id = dist.id;
            }
        }

        Ok(Some(State::CdnDns))
    }
}

struct CdnDnsTransition;

#[async_trait]
impl Transition for CdnDnsTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {

        let records = vec![
            DnsRecord::new_cloudfront_alias(
                request.domain_name.clone(),
                response.cdn_domain_name.clone(),
                RecordType::A,
            ),
            DnsRecord::new_cloudfront_alias(
                request.domain_name.clone(),
                response.cdn_domain_name.clone(),
                RecordType::AAAA,
            ),
        ];

        let dns = DnsSettings::new(response.zone_id.clone());
        let client = new_route53_client(&request.credentials)?;
        dns.upsert(&client, records).await?;

        if let Some(_) = response.redirect_bucket_endpoint {
            Ok(Some(State::RedirectCname))
        } else {
            Ok(None)
        }
    }
}

struct RedirectCnameTransition;

#[async_trait]
impl Transition for RedirectCnameTransition {
    async fn next(
        &self,
        request: &WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>> {

        let name =
            if let Some(ref domain) = request.redirect_domain_name {
                domain.clone()
            } else {
                format!("{}.{}", WWW, request.domain_name)
            };

        let value = format!("http://{}", response.redirect_bucket_endpoint.as_ref().unwrap());

        let records = vec![
            DnsRecord {
                name,
                value,
                kind: RecordType::CNAME,
                alias: None,
                ttl: Some(300),
            }
        ];

        let dns = DnsSettings::new(response.zone_id.clone());
        let client = new_route53_client(&request.credentials)?;
        dns.upsert(&client, records).await?;

        Ok(None)
    }
}

/// Ensure name servers are configured.
pub async fn ensure_domain(req: &WebHostRequest) -> Result<WebHostResponse> {
    Ok(run(req, &[State::NameServer]).await?)
}

/// Ensure all resources for a web host.
pub async fn ensure_website(req: &WebHostRequest) -> Result<WebHostResponse> {
    Ok(run(
        req,
        &[
            State::NameServer,
            State::HostedZone,
            State::Certificate,
            State::Bucket,
            State::RedirectBucket,
            State::Cdn,
            State::CdnDns,
            State::RedirectCname,
        ],
    )
    .await?)
}

/// Load the request from a source TOML file.
pub fn load_host_file<P: AsRef<Path>>(input: P) -> Result<WebHostRequest> {
    let input = input.as_ref().to_path_buf();
    let contents = fs::read_to_string(&input)?;
    let request: WebHostRequest = toml::from_str(&contents)?;
    Ok(request)
}

/// Run state transitions through to completion.
async fn run(
    req: &WebHostRequest,
    states: &[State],
) -> Result<WebHostResponse> {
    let mut res: WebHostResponse = Default::default();
    let mut machine = StateMachine::new(states);
    while let Some((state, transition)) = machine.next() {
        debug!("Current state {:?}", state);
        let next_state = transition.next(req, &mut res).await?;
        if let Some(state) = next_state {
            debug!("Advance state {:?}", state);
            machine.advance(state);
        } else {
            machine.stop();
        }
    }
    Ok(res)
}
