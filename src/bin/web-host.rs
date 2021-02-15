extern crate log;
extern crate pretty_env_logger;

use std::path::PathBuf;

use log::{info, warn};
use semver::Version;
use structopt::StructOpt;
use url::Url;

use rusoto_core::Region;

use uwe::{self, fatal, Error, Result};

use web_host::{
    ensure_domain, ensure_website, list_name_servers, load_host_file,
    rusoto_route53::CreateHostedZoneResponse, trim_hosted_zone_id,
    BucketSettings, CertSettings, CertUpsert, DistributionSettings, DnsRecord,
    DnsSettings, HostedZoneUpsert, RecordType, ViewerProtocolPolicy,
    WebHostRequest, ZoneSettings,
};

fn parse_region(src: &str) -> std::result::Result<Region, Error> {
    src.parse::<Region>().map_err(Error::from)
}

fn parse_url(src: &str) -> std::result::Result<Url, Error> {
    src.parse::<Url>().map_err(Error::from)
}

fn parse_policy(src: &str) -> std::result::Result<ViewerProtocolPolicy, Error> {
    src.parse::<ViewerProtocolPolicy>().map_err(Error::from)
}

fn parse_record_type(src: &str) -> std::result::Result<RecordType, Error> {
    src.parse::<RecordType>().map_err(Error::from)
}

fn log_zone_create(res: CreateHostedZoneResponse) {
    let id = trim_hosted_zone_id(&res.hosted_zone.id);
    /*
    for ns in res.delegation_set.name_servers.iter() {
        info!("Name server: {}", ns);
    }
    */
    info!("Created zone {} ({}) ✓", id, &res.hosted_zone.name);
}

/// Universal (web editor) plugin manager
#[derive(Debug, StructOpt)]
#[structopt(name = "upm")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
struct Common {
    /// Credentials profile name
    #[structopt(short, long)]
    credentials: String,
}

#[derive(StructOpt, Debug)]
enum Host {
    /// Make a bucket available
    Up {
        /// Suffix for folder requests
        #[structopt(short, long, default_value = "index.html")]
        index_suffix: String,

        /// Key for a bucket error handler
        #[structopt(short, long, default_value = "404.html")]
        error_key: String,

        /// Redirect all requests to the given host name
        #[structopt(long)]
        redirect_host_name: Option<String>,

        /// Protocol when redirecting all requests
        #[structopt(long)]
        redirect_protocol: Option<String>,

        #[structopt(flatten)]
        common: Common,

        /// Region for the bucket
        #[structopt(short, long, parse(try_from_str = parse_region))]
        region: Region,

        /// Bucket name
        bucket: String,
    },
}

#[derive(StructOpt, Debug)]
struct RecordInfo {
    /// Create Cloudfront alias record type
    #[structopt(long)]
    cdn: bool,

    /// The record type.
    #[structopt(short = "t", long = "type", parse(try_from_str = parse_record_type))]
    kind: RecordType,

    /// The name of the DNS record.
    name: String,

    /// The value of the DNS record.
    value: String,
}

impl Into<Vec<DnsRecord>> for RecordInfo {
    fn into(self) -> Vec<DnsRecord> {
        if self.cdn {
            vec![DnsRecord::new_cloudfront_alias(
                self.name, self.value, self.kind,
            )]
        } else {
            vec![DnsRecord {
                kind: self.kind,
                name: self.name,
                value: self.value,
                alias: None,
                ttl: Some(300),
            }]
        }
    }
}

#[derive(StructOpt, Debug)]
enum ZoneRecord {
    /// Create or update a record set
    Upsert {
        #[structopt(flatten)]
        record: RecordInfo,
    },
    /// Delete a record set
    Delete {
        #[structopt(flatten)]
        record: RecordInfo,
    },
}

#[derive(StructOpt, Debug)]
enum DnsZone {
    /// Create a hosted zone
    Create {
        /// The domain name for the zone
        domain_name: String,
    },
    /// Delete a hosted zone
    Delete {
        /// The identifier for the zone
        id: String,
    },
    /// Create a hosted zone if it does not exist
    Upsert {
        /// The domain name for the zone
        domain_name: String,
    },
}

#[derive(StructOpt, Debug)]
enum Dns {
    /// Manage DNS record sets
    Record {
        /// Hosted zone id.
        #[structopt(short, long)]
        zone_id: String,

        #[structopt(flatten)]
        common: Common,

        #[structopt(subcommand)]
        cmd: ZoneRecord,
    },
    /// Manage hosted zones
    Zone {
        #[structopt(flatten)]
        common: Common,

        #[structopt(subcommand)]
        cmd: DnsZone,
    },
}

#[derive(StructOpt, Debug)]
struct CertOptions {
    /// Hosted zone id for DNS validation
    #[structopt(short, long)]
    zone_id: String,

    /// Timeout in seconds
    #[structopt(short, long, default_value = "300")]
    timeout: u64,

    /// Monitor certificate status
    #[structopt(short, long)]
    monitor: bool,

    /// Alternative name(s) for the certificate (eg: *.example.com)
    #[structopt(short, long)]
    alternative_name: Option<Vec<String>>,

    /// Domain name for the certificate (eg: example.com)
    domain_name: String,
}

#[derive(StructOpt, Debug)]
enum Cert {
    /// Issue a certificate
    Issue {
        #[structopt(flatten)]
        common: Common,

        #[structopt(flatten)]
        options: CertOptions,
    },

    /// Create a certificate if it does not exist
    Upsert {
        #[structopt(flatten)]
        common: Common,

        #[structopt(flatten)]
        options: CertOptions,
    },

    /// Describe a certificate
    Describe {
        #[structopt(flatten)]
        common: Common,

        /// ARN identifier for the certificate
        arn: String,
    },

    /// Monitor a certificate status
    Monitor {
        /// Timeout in seconds
        #[structopt(short, long, default_value = "300")]
        timeout: u64,

        #[structopt(flatten)]
        common: Common,

        /// ARN identifier for the certificate
        arn: String,
    },
}

#[derive(StructOpt, Debug)]
enum Cdn {
    /// Create a Cloudfront CDN
    Upsert {
        #[structopt(flatten)]
        common: Common,

        /// CNAME aliases
        #[structopt(short, long)]
        alias: Vec<String>,

        /// Origin identifier
        #[structopt(short, long)]
        origin_id: Option<String>,

        /// Viewer protocol policy.
        #[structopt(long, parse(try_from_str = parse_policy), default_value = "allow-all")]
        protocol_policy: ViewerProtocolPolicy,

        /// Comment for the distribution.
        #[structopt(long)]
        comment: Option<String>,

        /// ACM certificate ARN
        #[structopt(long)]
        acm_certificate_arn: Option<String>,

        /// Origin URL
        #[structopt(parse(try_from_str = parse_url))]
        origin: Url,
    },
}

#[derive(StructOpt, Debug)]
enum Ensure {
    /// Ensure a domamin name has it's name
    /// servers configured correctly.
    Domain {
        /// The domain name to check.
        domain_name: String,
    },
    /// Ensure all resoures are configured
    /// for a website.
    Website {
        #[structopt(flatten)]
        common: Common,
        /// The website host file (TOML).
        #[structopt(parse(from_os_str))]
        host_file: PathBuf,
    },
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Ensure domamin and website resources
    #[structopt(alias = "up")]
    Ensure {
        #[structopt(subcommand)]
        cmd: Ensure,
    },

    /// Static website hosts (S3)
    #[structopt(alias = "bucket")]
    Host {
        #[structopt(subcommand)]
        cmd: Host,
    },

    /// Content distribution networks (Cloudfront)
    Cdn {
        #[structopt(subcommand)]
        cmd: Cdn,
    },

    /// DNS (Route53)
    Dns {
        #[structopt(subcommand)]
        cmd: Dns,
    },

    /// SSL certificates (ACM)
    Cert {
        #[structopt(subcommand)]
        cmd: Cert,
    },
}

async fn run(cmd: Command) -> Result<()> {
    match cmd {
        Command::Ensure { cmd } => match cmd {
            Ensure::Domain { domain_name } => {
                let req = WebHostRequest::new_domain(domain_name);
                match ensure_domain(&req).await {
                    Ok(_) => {
                        info!("Name servers ok ✓");
                    }
                    Err(e) => {
                        for ns in list_name_servers() {
                            warn!("Expecting NS record {}", ns);
                        }
                        return Err(Error::from(e));
                    }
                }
            }
            Ensure::Website { common, host_file } => {
                if !host_file.exists() {
                    return Err(Error::NotFile(host_file));
                }

                let mut req = load_host_file(&host_file)?;
                req.set_credentials(common.credentials);

                println!("Request {:?}", req);
                ensure_website(&req).await?;
            }
        },
        Command::Host { cmd } => match cmd {
            Host::Up {
                common,
                region,
                bucket,
                index_suffix,
                error_key,
                redirect_host_name,
                redirect_protocol,
            } => {
                let client =
                    web_host::new_s3_client(&common.credentials, &region)?;
                let bucket = BucketSettings::new(
                    region,
                    bucket,
                    index_suffix,
                    error_key,
                    redirect_host_name,
                    redirect_protocol,
                );
                bucket.up(&client).await?;
                info!("{} ✓", bucket.url());
            }
        },

        Command::Cdn { cmd } => match cmd {
            Cdn::Upsert {
                common,
                origin,
                origin_id,
                alias,
                acm_certificate_arn,
                protocol_policy,
                mut comment,
            } => {
                let client =
                    web_host::new_cloudfront_client(&common.credentials)?;
                let mut cdn =
                    DistributionSettings::new(origin, alias, origin_id);
                cdn.set_acm_certificate_arn(acm_certificate_arn);
                cdn.set_viewer_protocol_policy(protocol_policy);
                if let Some(comment) = comment.take() {
                    cdn.set_comment(comment);
                }
                cdn.upsert(&client).await?;
            }
        },
        Command::Cert { cmd } => match cmd {
            Cert::Upsert {
                common,
                options, /*
                         domain_name,
                         zone_id,
                         common,
                         alternative_name,
                         monitor,
                         timeout,
                         */
            } => {
                /*
                let alternative_name = if options.alternative_name.is_empty() {
                    None
                } else {
                    Some(alternative_name)
                };
                */
                let client = web_host::new_acm_client(&common.credentials)?;
                let dns_client =
                    web_host::new_route53_client(&common.credentials)?;
                let cert = CertSettings::new();
                match cert
                    .upsert(
                        &client,
                        &dns_client,
                        options.domain_name,
                        options.alternative_name,
                        options.zone_id,
                        options.monitor,
                        options.timeout,
                    )
                    .await?
                {
                    CertUpsert::Create(arn) => {
                        info!("Created certificate {} ✓", arn);
                    }
                    CertUpsert::Exists(detail) => {
                        info!(
                            "Certificate exists {} ✓",
                            detail.certificate_arn.unwrap()
                        );
                    }
                }
            }

            Cert::Issue {
                common,
                options, /*
                         domain_name,
                         zone_id,
                         common,
                         alternative_name,
                         monitor,
                         timeout,
                             */
            } => {
                /*
                let alternative_name = if options.alternative_name.is_empty() {
                    None
                } else {
                    Some(alternative_name)
                };
                */
                let client = web_host::new_acm_client(&common.credentials)?;
                let dns_client =
                    web_host::new_route53_client(&common.credentials)?;
                let cert = CertSettings::new();
                let arn = cert
                    .create(
                        &client,
                        &dns_client,
                        options.domain_name,
                        options.alternative_name,
                        options.zone_id,
                        options.monitor,
                        options.timeout,
                    )
                    .await?;

                info!("Created certificate {} ✓", arn);
            }

            Cert::Describe { arn, common } => {
                let client = web_host::new_acm_client(&common.credentials)?;
                let cert = CertSettings::new();
                let info = cert.describe_certificate(&client, arn).await?;
                info!("{:#?}", info);
            }

            Cert::Monitor {
                arn,
                common,
                timeout,
            } => {
                let client = web_host::new_acm_client(&common.credentials)?;
                let cert = CertSettings::new();
                let _ = cert
                    .monitor_certificate(&client, arn.clone(), timeout)
                    .await?;
                info!("Certificate issued {} ✓", arn);
            }
        },
        Command::Dns { cmd } => match cmd {
            Dns::Record {
                zone_id,
                common,
                cmd,
            } => match cmd {
                ZoneRecord::Delete { record } => {
                    let client =
                        web_host::new_route53_client(&common.credentials)?;
                    let dns = DnsSettings::new(zone_id);
                    let records: Vec<DnsRecord> = record.into();
                    for r in records.iter() {
                        info!(
                            "Delete record {} {} -> {}",
                            &r.kind, &r.name, &r.value
                        );
                    }
                    dns.delete(&client, records).await?;
                    info!("Deleted record(s) ✓");
                }
                ZoneRecord::Upsert { record } => {
                    let client =
                        web_host::new_route53_client(&common.credentials)?;
                    let dns = DnsSettings::new(zone_id);
                    let records: Vec<DnsRecord> = record.into();
                    for r in records.iter() {
                        info!(
                            "Upsert record {} {} -> {}",
                            &r.kind, &r.name, &r.value
                        );
                    }
                    dns.upsert(&client, records).await?;
                    info!("Upserted record(s) ✓");
                }
            },
            Dns::Zone { common, cmd } => match cmd {
                DnsZone::Upsert { domain_name } => {
                    let client =
                        web_host::new_route53_client(&common.credentials)?;
                    let zone = ZoneSettings::new();
                    match zone.upsert(&client, domain_name).await? {
                        HostedZoneUpsert::Create(res) => {
                            log_zone_create(res);
                        }
                        HostedZoneUpsert::Exists(res) => {
                            info!(
                                "Zone exists {} {} ",
                                res.name,
                                trim_hosted_zone_id(&res.id)
                            );
                        }
                    }
                }
                DnsZone::Create { domain_name } => {
                    let client =
                        web_host::new_route53_client(&common.credentials)?;
                    let zone = ZoneSettings::new();
                    let res = zone.create(&client, domain_name).await?;
                    log_zone_create(res);
                }
                DnsZone::Delete { id } => {
                    let client =
                        web_host::new_route53_client(&common.credentials)?;
                    let zone = ZoneSettings::new();
                    let res = zone.delete(&client, id).await?;
                    let id = res.change_info.id.trim_start_matches("/change/");
                    info!(
                        "Deleted zone {} ({}) ✓",
                        id, &res.change_info.status
                    );
                }
            },
        },
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    uwe::panic_hook();
    uwe::opts::log_level(&*args.log_level).or_else(fatal)?;

    // Configure the generator meta data ahead of time

    // Must configure the version here otherwise option_env!() will
    // use the version from the workspace package which we don't really
    // care about, the top-level version is the one that interests us.
    let name = env!("CARGO_PKG_NAME").to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let bin_name = env!("CARGO_BIN_NAME").to_string();
    let user_agent = format!("{}/{}", &name, &version);
    let semver: Version = version.parse().unwrap();

    info!("{}", &version);

    let app_data = config::generator::AppData {
        name,
        bin_name,
        version,
        user_agent,
        semver,
    };
    config::generator::get(Some(app_data));

    Ok(run(args.cmd).await.map_err(Error::from).or_else(fatal)?)
}
