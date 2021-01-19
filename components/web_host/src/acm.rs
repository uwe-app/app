use log::debug;

use rusoto_core::{credential, request::HttpClient, Region};
use rusoto_route53::Route53Client;
use rusoto_acm::{
    Acm, AcmClient, DescribeCertificateRequest, DescribeCertificateResponse,
    RequestCertificateRequest, RequestCertificateResponse,
};

use super::route53::{DnsRecord, DnsSettings};

use crate::{Error, Result};

// NOTE: Using certificates with cloudfront requires the region is US East (N Virginia).
// SEE: https://docs.aws.amazon.com/acm/latest/userguide/acm-regions.html

pub fn new_client(profile: &str) -> Result<AcmClient> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(profile);
    Ok(AcmClient::new_with(
        HttpClient::new()?,
        provider,
        Region::UsEast1,
    ))
}

static DNS: &str = "DNS";

#[derive(Debug)]
pub struct CertSettings {
    domain_name: String,
    subject_alternative_names: Option<Vec<String>>,
    validation_method: Option<String>,
}

impl CertSettings {
    pub fn new(
        domain_name: String,
        subject_alternative_names: Option<Vec<String>>,
    ) -> Self {
        Self {
            domain_name,
            subject_alternative_names,
            validation_method: Some(DNS.to_string()),
        }
    }

    /// Request a certificate and automatically add the domain validation requirements
    /// as DNS records to a hosted zone.
    pub async fn request_hosted_certificate(
        &self,
        client: &AcmClient,
        dns_client: &Route53Client,
        zone_id: String,
    ) -> Result<()> {
        debug!("Request certificate...");
        let res = self.request_certificate(client).await?;
        let certificate_arn =
            res.certificate_arn.ok_or_else(|| Error::NoCertificateArn)?;
        debug!("Describe certificate {}", &certificate_arn);
        let res = self.describe_certificate(client, certificate_arn).await?;

        debug!("Finding domain validation information...");
        if let Some(certificate) = res.certificate {
            if let Some(domain_validation_options) = certificate.domain_validation_options {
                let dns = DnsSettings::new(zone_id);
                for validation in domain_validation_options.iter() {
                    if let Some(ref resource_record) = validation.resource_record {
                        debug!("Create domain validation resource record {} {} {}",
                            resource_record.type_,
                            resource_record.name,
                            resource_record.value);

                        let records = vec![
                            DnsRecord {
                                name: resource_record.name.clone(), 
                                value: resource_record.value.clone(),
                                kind: resource_record.type_.parse()?,
                                alias: None,
                            }
                        ];

                        dns.create(dns_client, records).await?;
                    }
                } 
            } 
        }
        Ok(())
    }

    /// Request a certificate.
    async fn request_certificate(
        &self,
        client: &AcmClient,
    ) -> Result<RequestCertificateResponse> {
        let req = RequestCertificateRequest {
            domain_name: self.domain_name.clone(),
            subject_alternative_names: self.subject_alternative_names.clone(),
            validation_method: self.validation_method.clone(),
            idempotency_token: Some(self.domain_name.clone()),
            ..Default::default()
        };
        Ok(client.request_certificate(req).await?)
    }

    /// Describe a certificate.
    async fn describe_certificate(
        &self,
        client: &AcmClient,
        certificate_arn: String,
    ) -> Result<DescribeCertificateResponse> {
        let req = DescribeCertificateRequest { certificate_arn };
        Ok(client.describe_certificate(req).await?)
    }
}
