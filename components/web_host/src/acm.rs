use std::str::FromStr;
use std::{
    thread,
    time::{Duration, SystemTime},
};

use log::{debug, info};

use trust_dns_client::rr::domain::Name;

use rusoto_acm::{
    Acm, AcmClient, CertificateDetail, CertificateSummary,
    DescribeCertificateRequest, DescribeCertificateResponse,
    ListCertificatesRequest, ListCertificatesResponse,
    RequestCertificateRequest, RequestCertificateResponse,
};
use rusoto_core::{credential, request::HttpClient, Region};
use rusoto_route53::Route53Client;

use super::route53::{DnsRecord, DnsSettings};

use crate::{Error, Result};

const MAX_ITEMS: i64 = 100;
const DNS: &str = "DNS";

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

#[derive(Debug, strum_macros::Display, strum_macros::IntoStaticStr)]
pub enum CertificateValidationStatus {
    #[strum(to_string = "SUCCESS")]
    Success,
    #[strum(to_string = "FAILED")]
    Failed,
    #[strum(to_string = "PENDING_VALIDATION")]
    PendingValidation,
}

impl FromStr for CertificateValidationStatus {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "SUCCESS" => Ok(Self::Success),
            "FAILED" => Ok(Self::Failed),
            "PENDING_VALIDATION" => Ok(Self::PendingValidation),
            _ => Err(Error::UnknownValidationStatus(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub enum CertUpsert {
    Create(String),
    Exists(CertificateDetail),
}

#[derive(Debug)]
pub struct CertSettings {
    validation_method: Option<String>,
    idempotency_token: Option<String>,
}

impl CertSettings {
    pub fn new() -> Self {
        Self {
            validation_method: Some(DNS.to_string()),
            idempotency_token: None,
        }
    }

    /// Request a certificate if a certificate does not already exist
    /// that matches the given domain name and alternaitve names.
    pub async fn upsert(
        &self,
        client: &AcmClient,
        dns_client: &Route53Client,
        domain_name: String,
        subject_alternative_names: Option<Vec<String>>,
        zone_id: String,
        monitor: bool,
        timeout: u64,
    ) -> Result<CertUpsert> {
        let (ascii_name, alternative_names) =
            self.to_idna_punycode(domain_name, subject_alternative_names)?;

        // NOTE: When a certificate is described the subject alternative names
        // NOTE: already includes the primary domain name.
        let mut target_names = vec![ascii_name.clone()];
        if let Some(ref alternative_names) = alternative_names {
            target_names.extend(alternative_names.clone());
        }
        target_names.sort();

        let certs = self.list_all(client).await?;
        // NOTE: Must describe each certificate to get the subject_alternative_names
        for summary in certs {
            if let Some(certificate_arn) = summary.certificate_arn {
                let res =
                    self.describe_certificate(client, certificate_arn).await?;
                if let Some(mut certificate) = res.certificate {
                    if let Some(ref mut subject_names) =
                        certificate.subject_alternative_names
                    {
                        subject_names.sort();
                        if &mut target_names == subject_names {
                            return Ok(CertUpsert::Exists(certificate));
                        }
                    }
                }
            }
        }

        Ok(CertUpsert::Create(
            self.request_hosted_certificate(
                client,
                dns_client,
                ascii_name,
                alternative_names,
                zone_id,
                monitor,
                timeout,
            )
            .await?,
        ))
    }

    /// List all certificates.
    pub async fn list_all(
        &self,
        client: &AcmClient,
    ) -> Result<Vec<CertificateSummary>> {
        let mut out = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut result = self.list(client, next_token.clone()).await?;
            if let Some(certificate_summary_list) =
                result.certificate_summary_list.take()
            {
                out.extend(certificate_summary_list);
            }
            if let Some(token) = result.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }
        Ok(out)
    }

    /// List certificates until `MAX_ITEMS` is reached.
    pub async fn list(
        &self,
        client: &AcmClient,
        next_token: Option<String>,
    ) -> Result<ListCertificatesResponse> {
        let req = ListCertificatesRequest {
            next_token,
            max_items: Some(MAX_ITEMS),
            ..Default::default()
        };
        let res = client.list_certificates(req).await?;
        Ok(res)
    }

    /// Request a certificate and automatically add the domain validation requirements
    /// as DNS records to a hosted zone.
    ///
    /// The domain name and subject alternative names are converted from UTF8 to ASCII
    /// punycode.
    pub async fn create(
        &self,
        client: &AcmClient,
        dns_client: &Route53Client,
        domain_name: String,
        subject_alternative_names: Option<Vec<String>>,
        zone_id: String,
        monitor: bool,
        timeout: u64,
    ) -> Result<String> {
        let (ascii_name, alternative_names) =
            self.to_idna_punycode(domain_name, subject_alternative_names)?;
        info!("Request certificate for {}", &ascii_name);

        self.request_hosted_certificate(
            client,
            dns_client,
            ascii_name,
            alternative_names,
            zone_id,
            monitor,
            timeout,
        )
        .await
    }

    fn to_idna_punycode(
        &self,
        domain_name: String,
        subject_alternative_names: Option<Vec<String>>,
    ) -> Result<(String, Option<Vec<String>>)> {
        let idna_name = Name::from_utf8(&domain_name)?;
        let ascii_name = idna_name.to_ascii();
        let alternative_names = if let Some(subject_alternative_names) =
            subject_alternative_names
        {
            let mut out = Vec::new();
            for name in subject_alternative_names {
                out.push(Name::from_utf8(&name)?.to_ascii());
            }
            Some(out)
        } else {
            None
        };

        Ok((ascii_name, alternative_names))
    }

    /// Request a hosted certificate.
    ///
    /// Names must already have been converted to IDNA format.
    async fn request_hosted_certificate(
        &self,
        client: &AcmClient,
        dns_client: &Route53Client,
        domain_name: String,
        subject_alternative_names: Option<Vec<String>>,
        zone_id: String,
        monitor: bool,
        timeout: u64,
    ) -> Result<String> {
        let idna_name = Name::from_utf8(&domain_name)?;
        let ascii_name = idna_name.to_ascii();

        info!(
            "Request certificate for {} (IDNA: {})",
            &domain_name, &ascii_name
        );

        let alternative_names = if let Some(subject_alternative_names) =
            subject_alternative_names
        {
            let mut out = Vec::new();
            for name in subject_alternative_names {
                out.push(Name::from_utf8(&name)?.to_ascii());
            }
            Some(out)
        } else {
            None
        };

        let res = self
            .request_certificate(client, ascii_name, alternative_names)
            .await?;

        let certificate_arn =
            res.certificate_arn.ok_or_else(|| Error::NoCertificateArn)?;

        // NOTE: describing the certificate immediately does not always
        // NOTE: have the resource record in the domain validation options
        // NOTE: so we set up a loop for the resource records we need to
        // NOTE: become available
        info!("Wait for DNS validation records {}", &certificate_arn);
        self.wait_for_dns_validation(
            client,
            dns_client,
            zone_id,
            certificate_arn.clone(),
            30,
        )
        .await?;

        if monitor {
            let _ = self
                .monitor_certificate(&client, certificate_arn.clone(), timeout)
                .await?;
        }

        Ok(certificate_arn)
    }

    async fn wait_for_dns_validation(
        &self,
        client: &AcmClient,
        dns_client: &Route53Client,
        zone_id: String,
        arn: String,
        timeout: u64,
    ) -> Result<()> {
        let start = SystemTime::now();
        loop {
            if SystemTime::now().duration_since(start).unwrap()
                > Duration::from_secs(timeout)
            {
                return Err(Error::DnsValidationTimeout(timeout));
            }
            debug!("Finding domain validation information...");
            let res = self.describe_certificate(client, arn.clone()).await?;
            if let Some(certificate) = res.certificate {
                if let Some(domain_validation_options) =
                    certificate.domain_validation_options
                {
                    let dns = DnsSettings::new(zone_id.clone());
                    let validation = domain_validation_options
                        .iter()
                        .find(|v| v.validation_method == Some(DNS.to_string()));

                    if let Some(validation) = validation {
                        if let Some(ref resource_record) =
                            validation.resource_record
                        {
                            debug!("Create domain validation resource record {} {} {}",
                                resource_record.type_,
                                resource_record.name,
                                resource_record.value);

                            let records = vec![DnsRecord {
                                name: resource_record.name.clone(),
                                value: resource_record.value.clone(),
                                kind: resource_record.type_.parse()?,
                                alias: None,
                                ttl: Some(300),
                            }];

                            dns.upsert(dns_client, records).await?;
                            info!(
                                "Created validation record {} {} {}",
                                resource_record.type_,
                                resource_record.name,
                                resource_record.value
                            );
                            return Ok(());
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(50))
        }
    }

    /// Monitor the status of a certificate.
    pub async fn monitor_certificate(
        &self,
        client: &AcmClient,
        arn: String,
        timeout: u64,
    ) -> Result<()> {
        info!("Monitor {}", &arn);
        let start = SystemTime::now();

        loop {
            if SystemTime::now().duration_since(start).unwrap()
                > Duration::from_secs(timeout)
            {
                return Err(Error::MonitorTimeout(timeout));
            }
            let res = self.describe_certificate(client, arn.clone()).await?;
            if let Some(certificate) = res.certificate {
                if let Some(domain_validation_options) =
                    certificate.domain_validation_options
                {
                    let validation = domain_validation_options
                        .iter()
                        .find(|v| v.validation_method == Some(DNS.to_string()));

                    if let Some(validation) = validation {
                        if let Some(ref status) = validation.validation_status {
                            let validation_status: CertificateValidationStatus =
                                status.parse()?;
                            match validation_status {
                                CertificateValidationStatus::Success => {
                                    return Ok(())
                                }
                                CertificateValidationStatus::Failed => {
                                    return Err(
                                        Error::CertificateValidationFailed(
                                            arn.clone(),
                                        ),
                                    )
                                }
                                CertificateValidationStatus::PendingValidation => {
                                    if let Ok(t) = start.elapsed() {
                                        info!("Pending validation, elapsed: {:?} (timeout: {}s)", t, timeout);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_secs(15))
        }
    }

    /// Request a certificate.
    async fn request_certificate(
        &self,
        client: &AcmClient,
        domain_name: String,
        subject_alternative_names: Option<Vec<String>>,
    ) -> Result<RequestCertificateResponse> {
        let req = RequestCertificateRequest {
            domain_name: domain_name.clone(),
            subject_alternative_names: subject_alternative_names.clone(),
            validation_method: self.validation_method.clone(),
            idempotency_token: self.idempotency_token.clone(),
            ..Default::default()
        };
        Ok(client.request_certificate(req).await?)
    }

    /// Describe a certificate.
    pub async fn describe_certificate(
        &self,
        client: &AcmClient,
        certificate_arn: String,
    ) -> Result<DescribeCertificateResponse> {
        let req = DescribeCertificateRequest { certificate_arn };
        Ok(client.describe_certificate(req).await?)
    }
}
