//! Uses the DNS client to determine if a domain has
//! it's NS records set to our servers.

use crate::{dns_client, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use trust_dns_client::{
    op::DnsResponse,
    rr::{Name, RData},
};

static NS1: &str = "ns1.uwe.app.";
static NS2: &str = "ns2.uwe.app.";
static NS3: &str = "ns3.uwe.app.";
static NS4: &str = "ns4.uwe.app.";

#[derive(Debug)]
pub struct NameServerResult {
    responses: HashMap<SocketAddr, DnsResponse>,
    names: Vec<Name>,
}

impl NameServerResult {
    pub fn new() -> Self {
        let names = vec![
            Name::from_str(NS1).unwrap(),
            Name::from_str(NS2).unwrap(),
            Name::from_str(NS3).unwrap(),
            Name::from_str(NS4).unwrap(),
        ];

        Self {
            responses: HashMap::new(),
            names,
        }
    }

    /// Determine if all responses resolve to
    /// our expected name servers.
    pub fn is_propagated(&self) -> bool {
        let required_answers = self.responses.len() * self.names.len();
        let mut results = Vec::new();
        for (_, res) in self.responses.iter() {
            for (answer, expected) in
                res.answers().iter().zip(self.names.iter())
            {
                if let &RData::NS(ref name) = answer.rdata() {
                    results.push(name == expected);
                }
            }
        }
        results.len() == required_answers
    }
}

pub fn list() -> Vec<&'static str> {
    vec![NS1, NS2, NS3, NS4]
}

/// Ensure a domain name is qualified for a DNS lookup using
/// a trailing period.
///
/// This allows callers to specify with or without a trailing period.
pub fn qualified(domain_name: &str) -> String {
    if !domain_name.ends_with(".") {
        return format!("{}.", domain_name);
    }
    domain_name.to_string()
}

pub async fn lookup(fqdn: &str) -> Result<NameServerResult> {
    let resolvers: Vec<SocketAddr> = vec![
        // Open DNS (1)
        ([208, 67, 222, 222], 53).into(),
        // Open DNS (2)
        ([208, 67, 222, 220], 53).into(),
        // Cloudflare (1)
        ([1, 1, 1, 1], 53).into(),
        // Cloudflare (2)
        ([1, 0, 0, 1], 53).into(),
        // Google (1)
        ([8, 8, 8, 8], 53).into(),
        // Google (2)
        ([8, 8, 4, 4], 53).into(),
    ];

    let mut result = NameServerResult::new();
    for addr in resolvers {
        result
            .responses
            .insert(addr, dns_client::query_ns(addr, fqdn).await?);
    }
    Ok(result)
}
