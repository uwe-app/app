use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown viewer protocol policy {0}")]
    UnknownViewerProtocolPolicy(String),

    #[error("Unknown record type {0}")]
    UnknownDnsRecordType(String),

    #[error("Unknown certificate validation status {0}")]
    UnknownValidationStatus(String),

    #[error("Unable to find a cache policy matching the name {0}")]
    NoCachePolicy(String),

    #[error("Unable to get certificate ARN from response")]
    NoCertificateArn,

    #[error("Unable to get DNS record for certificate validation in timeout period of {0} seconds")]
    DnsValidationTimeout(u64),

    #[error(
        "Certificate status monitor exceeded timeout period of {0} seconds"
    )]
    MonitorTimeout(u64),

    #[error("Certificate validation failed for {0}")]
    CertificateValidationFailed(String),

    #[error("Name servers have not propagated yet; ensure they have been set and try again later")]
    NameServerPropagation,

    #[error(transparent)]
    Tls(#[from] rusoto_core::request::TlsError),

    #[error(transparent)]
    Credentials(#[from] rusoto_core::credential::CredentialsError),

    #[error(transparent)]
    HeadBucket(#[from] rusoto_core::RusotoError<rusoto_s3::HeadBucketError>),

    #[error(transparent)]
    CreateBucket(
        #[from] rusoto_core::RusotoError<rusoto_s3::CreateBucketError>,
    ),

    #[error(transparent)]
    PutBucketPolicy(
        #[from] rusoto_core::RusotoError<rusoto_s3::PutBucketPolicyError>,
    ),

    #[error(transparent)]
    PutBucketWebsite(
        #[from] rusoto_core::RusotoError<rusoto_s3::PutBucketWebsiteError>,
    ),

    #[error(transparent)]
    PutPublicAccessBlock(
        #[from] rusoto_core::RusotoError<rusoto_s3::PutPublicAccessBlockError>,
    ),

    #[error(transparent)]
    ListDistributions(
        #[from]
        rusoto_core::RusotoError<rusoto_cloudfront::ListDistributionsError>,
    ),

    #[error(transparent)]
    CreateDistribution(
        #[from]
        rusoto_core::RusotoError<rusoto_cloudfront::CreateDistributionError>,
    ),

    #[error(transparent)]
    UpdateDistribution(
        #[from]
        rusoto_core::RusotoError<rusoto_cloudfront::UpdateDistributionError>,
    ),

    #[error(transparent)]
    ListCachePolicies(
        #[from]
        rusoto_core::RusotoError<rusoto_cloudfront::ListCachePoliciesError>,
    ),

    #[error(transparent)]
    ChangeResourceRecordSets(
        #[from]
        rusoto_core::RusotoError<
            rusoto_route53::ChangeResourceRecordSetsError,
        >,
    ),

    #[error(transparent)]
    CreateHostedZone(
        #[from] rusoto_core::RusotoError<rusoto_route53::CreateHostedZoneError>,
    ),

    #[error(transparent)]
    DeleteHostedZone(
        #[from] rusoto_core::RusotoError<rusoto_route53::DeleteHostedZoneError>,
    ),

    #[error(transparent)]
    ListHostedZone(
        #[from] rusoto_core::RusotoError<rusoto_route53::ListHostedZonesError>,
    ),

    #[error(transparent)]
    RequestCertificate(
        #[from] rusoto_core::RusotoError<rusoto_acm::RequestCertificateError>,
    ),

    #[error(transparent)]
    DescribeCertificate(
        #[from] rusoto_core::RusotoError<rusoto_acm::DescribeCertificateError>,
    ),

    #[error(transparent)]
    Proto(#[from] trust_dns_client::proto::error::ProtoError),

    #[error(transparent)]
    DnsClient(#[from] trust_dns_client::error::ClientError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),
}
