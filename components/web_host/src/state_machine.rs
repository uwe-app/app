use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use async_trait::async_trait;
use rusoto_core::Region;

use crate::Result;

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

#[async_trait]
trait StateIterator {
    type Item;
    async fn next(&mut self) -> Result<Option<Self::Item>>;
}

#[derive(Debug)]
pub enum State {
    Empty,
    HostedZone,
    NameServer,
    Certificate,
    Bucket,
    RedirectBucket,
    Cdn,
    Dns4,
    Dns6,
    RedirectCname,
}

#[derive(Debug)]
pub struct StateMachine<'a> {
    request: &'a WebHostRequest, 
    response: &'a mut WebHostResponse,
    states: &'a [State],
    index: usize,
}

impl<'a> StateMachine<'a> {
    pub fn new(
        request: &'a WebHostRequest,
        response: &'a mut WebHostResponse,
        states: &'a [State]) -> Self {
        Self {request, response, states, index: 0} 
    }
}

#[async_trait]
impl StateIterator for StateMachine<'_> {
    type Item = State;

    async fn next(&mut self) -> Result<Option<State>> {
        if let Some(state) = self.states.get(self.index) {
            let item = match state {
                State::Empty => Some(State::HostedZone),
                State::HostedZone => Some(State::NameServer),
                State::NameServer => Some(State::Certificate),
                State::Certificate => Some(State::Bucket),
                State::Bucket => Some(State::RedirectBucket),
                State::RedirectBucket => Some(State::Cdn),
                State::Cdn => Some(State::Dns4),
                State::Dns4 => Some(State::Dns6),
                State::Dns6 => Some(State::RedirectCname),
                _ => None,
            };

            self.index += 1; 
            Ok(item)
        } else { Ok(None) }
    }
}

#[derive(Debug)]
pub struct WebHost;

impl WebHost {

    /// Ensure all resources for a web host.
    pub async fn ensure(req: &WebHostRequest) -> Result<WebHostResponse> {
        let mut res: WebHostResponse = Default::default();

        let mut machine = StateMachine::new(&req, &mut res,
            &[
                State::Empty,
                State::HostedZone,
                State::NameServer,
                State::Certificate,
                State::Bucket,
                State::RedirectBucket,
                State::Cdn,
                State::Dns4,
                State::Dns6,
                State::RedirectCname,
            ]
        );

        while let Some(state) = machine.next().await? {
            println!("Got a state {:?}", state);
        }

        Ok(res)
    }

    /// Load the request from a source TOML file.
    pub fn load<P: AsRef<Path>>(input: P) -> Result<WebHostRequest> {
        let input = input.as_ref().to_path_buf();
        let contents = fs::read_to_string(&input)?;
        let request: WebHostRequest = toml::from_str(&contents)?;
        Ok(request)
    }
}
