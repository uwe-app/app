use std::fmt;
use std::str::FromStr;

use rusoto_core::{credential, request::HttpClient, Region};
use rusoto_route53::{
    AliasTarget, Change, ChangeBatch, ChangeResourceRecordSetsRequest,
    ChangeResourceRecordSetsResponse as Response, CreateHostedZoneRequest,
    CreateHostedZoneResponse, DeleteHostedZoneRequest,
    DeleteHostedZoneResponse, HostedZone, ListHostedZonesRequest,
    ListHostedZonesResponse, ResourceRecord, ResourceRecordSet, Route53,
    Route53Client,
};

use crate::{list_name_servers, to_idna_punycode, Error, Result};

const MAX_ITEMS: usize = 100;
const DELEGATION_SET_ID: &str = "N02886841KKW7QD2MZLTC";
const SOA: &str = "ns1.uwe.app. dev.uwe.app. 1 7200 900 1209600 86400";

// Route53 must use the US East (N Virginia) region.
pub fn new_client(profile: &str) -> Result<Route53Client> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    Ok(Route53Client::new_with(
        HttpClient::new()?,
        provider,
        Region::UsEast1,
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
    /// TTL for the record.
    pub ttl: Option<i64>,
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
            ttl: Some(300),
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

        let ttl: Option<i64> = self.ttl.clone();

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
                // Records of type NS use a newline delimiter
                // and should be expanded to multiple resource record
                // values
                if value.contains("\n") {
                    let records: Vec<ResourceRecord> = value
                        .split("\n")
                        .map(|value| ResourceRecord {
                            value: value.to_string(),
                        })
                        .collect();
                    (None, Some(records))
                } else {
                    (None, Some(vec![ResourceRecord { value }]))
                }
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
pub enum HostedZoneUpsert {
    Create(CreateHostedZoneResponse),
    Exists(HostedZone),
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
        let ascii_name = to_idna_punycode(&name)?;
        let req = CreateHostedZoneRequest {
            caller_reference,
            delegation_set_id: Some(DELEGATION_SET_ID.to_string()),
            name: ascii_name.to_string(),
            ..Default::default()
        };
        let res = client.create_hosted_zone(req).await?;
        self.assign_name_servers(client, &res.hosted_zone.id, &ascii_name)
            .await?;
        Ok(res)
    }

    async fn assign_name_servers(
        &self,
        client: &Route53Client,
        zone_id: &str,
        idna_name: &str,
    ) -> Result<()> {
        let dns = DnsSettings::new(zone_id.to_string());

        let ns_value = list_name_servers().join("\n");

        // SOA and NS records for the new hosted zone
        // should use out name servers
        let records = vec![
            DnsRecord {
                kind: RecordType::SOA,
                name: idna_name.to_string(),
                value: SOA.to_string(),
                alias: None,
                ttl: Some(900),
            },
            DnsRecord {
                kind: RecordType::NS,
                name: idna_name.to_string(),
                value: ns_value,
                alias: None,
                ttl: Some(172800),
            },
        ];

        dns.upsert(client, records).await?;
        Ok(())
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

    /// Create a new hosted zone if it does not exist.
    pub async fn upsert(
        &self,
        client: &Route53Client,
        name: String,
    ) -> Result<HostedZoneUpsert> {
        use crate::name_servers;

        let ascii_name = to_idna_punycode(&name)?;
        let qualified_name = name_servers::qualified(&ascii_name);

        let zones = self.list_all(client).await?;
        let existing_zone = zones.iter().find(|z| &z.name == &qualified_name);
        if let Some(hosted_zone) = existing_zone {
            self.assign_name_servers(client, &hosted_zone.id, &ascii_name)
                .await?;
            Ok(HostedZoneUpsert::Exists(hosted_zone.clone()))
        } else {
            Ok(HostedZoneUpsert::Create(self.create(client, name).await?))
        }
    }

    /// List all hosted zones.
    pub async fn list_all(
        &self,
        client: &Route53Client,
    ) -> Result<Vec<HostedZone>> {
        let mut out = Vec::new();
        let mut marker: Option<String> = None;
        loop {
            let result = self.list(client, marker.clone()).await?;
            out.extend(result.hosted_zones);
            if !result.is_truncated {
                break;
            } else {
                marker = result.next_marker.clone();
            }
        }
        Ok(out)
    }

    /// List hosted zones until `MAX_ITEMS` is reached.
    pub async fn list(
        &self,
        client: &Route53Client,
        marker: Option<String>,
    ) -> Result<ListHostedZonesResponse> {
        let req = ListHostedZonesRequest {
            marker,
            max_items: Some(MAX_ITEMS.to_string()),
            ..Default::default()
        };
        let res = client.list_hosted_zones(req).await?;
        Ok(res)
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

    /*
    async fn create(
        &self,
        client: &Route53Client,
        mut records: Vec<DnsRecord>,
    ) -> Result<Response> {

        for mut r in records.iter_mut() {
            r.name = to_idna_punycode(&r.name)?;
        }

        let changes = records
            .into_iter()
            .map(|record| (ChangeAction::Create, record.into()))
            .collect();
        let req = self.into_change_set(changes);
        Ok(client.change_resource_record_sets(req).await?)
    }
    */

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
        mut records: Vec<DnsRecord>,
    ) -> Result<Response> {
        for mut r in records.iter_mut() {
            r.name = to_idna_punycode(&r.name)?;
        }

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
