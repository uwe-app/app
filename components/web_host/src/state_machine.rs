use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use async_trait::async_trait;
use rusoto_core::Region;

use crate::{name_servers, Error, Result};

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
    subject_alternative_names: Vec<String>,
    /// Name for the S3 bucket.
    bucket_name: String,
    /// Redirect all requests from this bucket to the primary
    /// bucket. Useful for configuring `www` to redirect to the
    /// primary domain.
    redirect_bucket_name: Option<String>,
}

impl WebHostRequest {
    /// If we are just checking the domamin name servers no
    /// AWS calls are required.
    pub fn new_domain(domain_name: String) -> Self {
        Self {
            credentials: String::new(),
            region: Region::UsEast1,
            bucket_name: domain_name.clone(),
            subject_alternative_names: Vec::new(),
            redirect_bucket_name: None,
            domain_name,
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
            subject_alternative_names: Vec::new(),
            redirect_bucket_name: None,
            domain_name: String::new(),
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
    /// Endpoint for the primary bucket.
    pub bucket_endpoint: String,
    /// Endpoint for the redirect bucket.
    pub redirect_bucket_endpoint: String,
    /// Domain name for the CDN.
    pub cdn_domain_name: String,
}

#[async_trait]
trait StateIterator {
    type Item;
    async fn next(&mut self) -> Result<Option<Self::Item>>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum State {
    NameServer,
    HostedZone,
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
            self.index = self.states.len()
        }
    }
}

#[async_trait]
pub trait Transition<'a> {
    async fn to(
        &self,
        request: &'a WebHostRequest,
        response: &mut WebHostResponse,
    ) -> Result<Option<State>>;
}

struct NameServerTransition;

#[async_trait]
impl<'a> Transition<'a> for NameServerTransition {
    async fn to(
        &self,
        request: &'a WebHostRequest,
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

/// Iterator yields a transition for a state.
impl<'a> Iterator for StateMachine<'a> {
    type Item = Box<dyn Transition<'a>>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(state) = self.states.get(self.index) {
            let item = match state {
                State::NameServer => {
                    let transition: Box<dyn Transition<'a>> =
                        Box::new(NameServerTransition {});
                    Some(transition)
                }
                /*
                State::HostedZone => Some(State::Certificate),
                State::Certificate => Some(State::Bucket),
                State::Bucket => Some(State::RedirectBucket),
                State::RedirectBucket => Some(State::Cdn),
                State::Cdn => Some(State::Dns4),
                State::Dns4 => Some(State::Dns6),
                State::Dns6 => Some(State::RedirectCname),
                */
                _ => None,
            };
            item
        } else {
            None
        }
    }
}

/// Ensure name servers are configured.
pub async fn ensure_domain(
    req: &WebHostRequest,
) -> Result<WebHostResponse> {
    Ok(run(req, &[State::NameServer]).await?)
}

/// Ensure all resources for a web host.
pub async fn ensure_website(
    req: &WebHostRequest,
) -> Result<WebHostResponse> {
    Ok(run(
        req,
        &[
            State::NameServer,
            State::HostedZone,
            State::Certificate,
            State::Bucket,
            State::RedirectBucket,
            State::Cdn,
            State::Dns4,
            State::Dns6,
            State::RedirectCname,
        ],
    )
    .await?)
}

/// Load the request from a source TOML file.
pub fn load_host_file<P: AsRef<Path>>(
    input: P,
) -> Result<WebHostRequest> {
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
    while let Some(transition) = machine.next() {
        let next_state = transition.to(req, &mut res).await?;
        if let Some(state) = next_state {
            machine.advance(state);
        }
    }
    Ok(res)
}
