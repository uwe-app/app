//use serde::{Deserialize, Serialize};
//use serde_with::{serde_as, DisplayFromStr};
use std::fmt;
use std::str::FromStr;

use rusoto_core::{credential, request::HttpClient, Region};
use rusoto_route53::{
    AliasTarget, Change, ChangeBatch, ChangeResourceRecordSetsRequest,
    ChangeResourceRecordSetsResponse as Response, CreateHostedZoneRequest,
    CreateHostedZoneResponse,
    DeleteHostedZoneRequest, DeleteHostedZoneResponse, ResourceRecord,
    ResourceRecordSet, Route53, Route53Client,
};

use crate::{Error, Result};

pub fn new_client(profile: &str, region: &Region) -> Result<Route53Client> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    Ok(Route53Client::new_with(
        HttpClient::new()?,
        provider,
        region.clone(),
    ))
}

#[derive(Debug, strum_macros::Display)]
pub enum RecordType {
    A,
    AAAA,
    CAA,
    CNAME,
    MX,
    NAPTR,
    NS,
    PTR,
    SOA,
    SPF,
    SRV,
    TXT,
}

impl FromStr for RecordType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "A" => Ok(Self::A),
            "AAAA" => Ok(Self::AAAA),
            "CAA" => Ok(Self::CAA),
            "CNAME" => Ok(Self::CNAME),
            "MX" => Ok(Self::MX),
            "NAPTR" => Ok(Self::NAPTR),
            "NS" => Ok(Self::NS),
            "PTR" => Ok(Self::PTR),
            "SOA" => Ok(Self::SOA),
            "SPF" => Ok(Self::SPF),
            "SRV" => Ok(Self::SRV),
            "TXT" => Ok(Self::TXT),
            _ => Err(Error::UnknownDnsRecordType(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct DnsRecord {
    /// The name of the record.
    pub name: String,
    /// The value of the record.
    pub value: String,
    /// The type of the record.
    pub kind: RecordType,
    /// A hosted zone id when an alias should be used.
    pub alias: Option<String>,
}

impl DnsRecord {
    pub fn new_cloudfront_alias(
        name: String,
        value: String,
        kind: RecordType,
    ) -> Self {
        Self {
            alias: Some("Z2FDTNDATAQYW2".to_string()),
            name,
            value,
            kind,
        }
    }
}

impl Into<ResourceRecordSet> for DnsRecord {
    fn into(self) -> ResourceRecordSet {
        let value = if let RecordType::TXT = self.kind {
            rusoto_route53::util::quote_txt_record(&self.value)
        } else {
            self.value
        };

        let ttl: Option<i64> = if let None = self.alias {
            Some(300) 
        } else { None };

        let (alias_target, resource_records) =
            if let Some(hosted_zone_id) = self.alias {
                (
                    Some(AliasTarget {
                        hosted_zone_id,
                        dns_name: value,
                        ..Default::default()
                    }),
                    None,
                )
            } else {
                (None, Some(vec![ResourceRecord { value }]))
            };

        ResourceRecordSet {
            name: self.name,
            type_: self.kind.to_string(),
            alias_target,
            resource_records,
            ttl,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
enum ChangeAction {
    Create,
    Delete,
    Upsert,
}

impl fmt::Display for ChangeAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Create => "CREATE",
                Self::Delete => "DELETE",
                Self::Upsert => "UPSERT",
            }
        )
    }
}

#[derive(Debug)]
pub struct ZoneSettings;

impl ZoneSettings {
    pub fn new() -> Self {
        Self {}
    }

    /// Create a new hosted zone.
    pub async fn create(
        &self,
        client: &Route53Client,
        name: String,
    ) -> Result<CreateHostedZoneResponse> {
        let caller_reference = utils::generate_id(16);
        let req = CreateHostedZoneRequest {
            caller_reference,
            name,
            ..Default::default()
        };
        Ok(client.create_hosted_zone(req).await?)
    }

    /// Delete a hosted zone.
    pub async fn delete(
        &self,
        client: &Route53Client,
        id: String,
    ) -> Result<DeleteHostedZoneResponse> {
        let req = DeleteHostedZoneRequest { id };
        Ok(client.delete_hosted_zone(req).await?)
    }
}

#[derive(Debug)]
pub struct DnsSettings {
    zone_id: String,
}

impl DnsSettings {
    pub fn new(zone_id: String) -> Self {
        Self { zone_id }
    }

    pub async fn create(
        &self,
        client: &Route53Client,
        records: Vec<DnsRecord>,
    ) -> Result<Response> {
        let changes = records
            .into_iter()
            .map(|record| (ChangeAction::Create, record.into()))
            .collect();
        let req = self.into_change_set(changes);
        Ok(client.change_resource_record_sets(req).await?)
    }

    pub async fn delete(
        &self,
        client: &Route53Client,
        records: Vec<DnsRecord>,
    ) -> Result<Response> {
        let changes = records
            .into_iter()
            .map(|record| (ChangeAction::Delete, record.into()))
            .collect();
        let req = self.into_change_set(changes);
        Ok(client.change_resource_record_sets(req).await?)
    }

    pub async fn upsert(
        &self,
        client: &Route53Client,
        records: Vec<DnsRecord>,
    ) -> Result<Response> {
        let changes = records
            .into_iter()
            .map(|record| (ChangeAction::Upsert, record.into()))
            .collect();
        let req = self.into_change_set(changes);
        Ok(client.change_resource_record_sets(req).await?)
    }

    fn into_change_set(
        &self,
        changes: Vec<(ChangeAction, ResourceRecordSet)>,
    ) -> ChangeResourceRecordSetsRequest {
        let change_batch = ChangeBatch {
            changes: changes
                .into_iter()
                .map(|(action, resource_record_set)| Change {
                    action: action.to_string(),
                    resource_record_set,
                })
                .collect(),
            ..Default::default()
        };

        ChangeResourceRecordSetsRequest {
            hosted_zone_id: self.zone_id.clone(),
            change_batch,
        }
    }
}
