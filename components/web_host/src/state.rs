use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use async_trait::async_trait;
use rusoto_core::Region;

use crate::Result;

#[async_trait]
trait AsyncState<I, O> {
    async fn transition(
        val: StateMachine<I>,
        request: &WebHostRequest, 
        response: &mut WebHostResponse
    ) -> Result<StateMachine<O>>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WebHostRequest {
    /// Name of the AWS credentials profile to use.
    credentials: String,
    /// Region for resources.
    region: Region,
    /// Domain name to host.
    domain_name: String,
    /// Alternative domain names for the SSL certificate.
    subject_alternative_names: Vec<String>,
    /// Name for the S3 bucket.
    bucket_name: String,
    /// Redirect all requests from this bucket to the primary
    /// bucket. Useful for configuring `www` to redirect to the
    /// primary domain.
    redirect_bucket_name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WebHostResponse {
    /// Hosted zone id.
    zone_id: String,
    /// List of name servers for the hosted zone.
    name_servers: Vec<String>,
    /// Whether the name servers have propagated.
    propagated: bool,
    /// An ARN identifier for the SSL certificate.
    certificate_arn: String,
    /// Endpoint for the primary bucket.
    bucket_endpoint: String,
    /// Endpoint for the redirect bucket.
    redirect_bucket_endpoint: String,
    /// Domain name for the CDN.
    cdn_domain_name: String,
}

impl Default for WebHostRequest {
    fn default() -> Self {
        Self {
            credentials: String::new(),
            region: Region::ApSoutheast1,
            domain_name: String::new(),
            subject_alternative_names: Vec::new(),
            bucket_name: String::new(),
            redirect_bucket_name: None,
        }
    }
}

#[derive(Debug)]
struct StateMachine<S> {
    state: S,
}

#[derive(Debug)]
struct EmptyState;

impl StateMachine<EmptyState> {
    fn new() -> Self {
        StateMachine { state: EmptyState }
    }
}

#[derive(Debug)]
struct HostedZoneState;

#[async_trait]
impl AsyncState<EmptyState, HostedZoneState> for StateMachine<HostedZoneState> {
    async fn transition(
        val: StateMachine<EmptyState>,
        request: &WebHostRequest,
        response: &mut WebHostResponse, 
    ) -> Result<StateMachine<HostedZoneState>> {

        // TODO: response.zone_id
        // TODO: response.name_servers

        Ok(StateMachine {state: HostedZoneState})
    }
}

#[derive(Debug)]
struct NameServerState;

#[async_trait]
impl AsyncState<HostedZoneState, NameServerState>
    for StateMachine<NameServerState>
{
    async fn transition(
        val: StateMachine<HostedZoneState>,
        request: &WebHostRequest,
        response: &mut WebHostResponse, 
    ) -> Result<StateMachine<NameServerState>> {

        // TODO: response.propagated

        Ok(StateMachine {state: NameServerState})
    }
}

#[derive(Debug)]
struct CertificateState;

#[async_trait]
impl AsyncState<NameServerState, CertificateState>
    for StateMachine<CertificateState>
{
    async fn transition(
        val: StateMachine<NameServerState>,
        request: &WebHostRequest,
        response: &mut WebHostResponse, 
    ) -> Result<StateMachine<CertificateState>> {

        // TODO: response.certificate_arn
        Ok(StateMachine {state: CertificateState})
    }
}

#[derive(Debug)]
struct BucketState;

#[async_trait]
impl AsyncState<CertificateState, BucketState>
    for StateMachine<BucketState>
{
    async fn transition(
        val: StateMachine<CertificateState>,
        request: &WebHostRequest,
        response: &mut WebHostResponse, 
    ) -> Result<StateMachine<BucketState>> {

        // TODO: response.bucket_endpoint
        Ok(StateMachine {state: BucketState})
    }
}

#[derive(Debug)]
pub struct WebHost;

impl WebHost {
    pub async fn ensure(req: &WebHostRequest) -> Result<()> {
        let mut res: WebHostResponse = Default::default();

        let empty = StateMachine::new();
        let hosted_zone =
            StateMachine::<HostedZoneState>::transition(empty, req, &mut res).await?;
        let name_server =
            StateMachine::<NameServerState>::transition(hosted_zone, req, &mut res).await?;
        let certificate =
            StateMachine::<CertificateState>::transition(name_server, req, &mut res).await?;
        let bucket =
            StateMachine::<BucketState>::transition(certificate, req, &mut res).await?;

        Ok(())
    }

    /// Load the request from a source TOML file.
    pub fn load<P: AsRef<Path>>(input: P) -> Result<WebHostRequest> {
        let input = input.as_ref().to_path_buf();
        let contents = fs::read_to_string(&input)?;
        let request: WebHostRequest = toml::from_str(&contents)?;
        Ok(request)
    }
}
