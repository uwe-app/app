use std::collections::HashMap;
use once_cell::sync::Lazy;

use rusoto_core::Region;

#[derive(Debug)]
pub struct RegionInfo {
    /// Human friendly name for the region
    pub title: String,
    /// URL suffix for S3 endpoints
    pub s3_endpoint_suffix: String,
    /// Route 53 zone id
    pub zone_id: Option<String>,
}

// SEE: https://docs.aws.amazon.com/AmazonS3/latest/dev/WebsiteEndpoints.html
// SEE: https://docs.aws.amazon.com/general/latest/gr/s3.html#s3_website_region_endpoints
pub static REGION_INFO: Lazy<HashMap<Region, RegionInfo>> = Lazy::new(|| {
    let mut m = HashMap::new();

    m.insert(Region::ApEast1, RegionInfo {
        title: "Asia Pacific (Hong Kong)".to_string(),
        s3_endpoint_suffix: "s3-website.ap-east-1.amazonaws.com".to_string(),
        zone_id: Some("ZNB98KWMFR0R6".to_string()),
    });

    m.insert(Region::ApNortheast1, RegionInfo {
        title: "Asia Pacific (Tokyo)".to_string(),
        s3_endpoint_suffix: "s3-website-ap-northeast-1.amazonaws.com".to_string(),
        zone_id: Some("Z2M4EHUR26P7ZW".to_string()),
    });

    m.insert(Region::ApNortheast2, RegionInfo {
        title: "Asia Pacific (Seoul)".to_string(),
        s3_endpoint_suffix: "s3-website.ap-northeast-2.amazonaws.com".to_string(),
        zone_id: Some("Z3W03O7B5YMIYP".to_string()),
    });

    m.insert(Region::ApNortheast3, RegionInfo {
        title: "Asia Pacific (Osaka-Local)".to_string(),
        s3_endpoint_suffix: "s3-website.ap-northeast-3.amazonaws.com".to_string(),
        zone_id: Some("Z2YQB5RD63NC85".to_string()),
    });

    m.insert(Region::ApSouth1, RegionInfo {
        title: "Asia Pacific (Mumbai)".to_string(),
        s3_endpoint_suffix: "s3-website.ap-south-1.amazonaws.com".to_string(),
        zone_id: Some("Z11RGJOFQNVJUP".to_string()),
    });

    m.insert(Region::ApSoutheast1, RegionInfo {
        title: "Asia Pacific (Singapore)".to_string(),
        s3_endpoint_suffix: "s3-website-ap-southeast-1.amazonaws.com".to_string(),
        zone_id: Some("Z3O0J2DXBE1FTB".to_string()),
    });

    m.insert(Region::ApSoutheast2, RegionInfo {
        title: "Asia Pacific (Sydney)".to_string(),
        s3_endpoint_suffix: "s3-website-ap-southeast-2.amazonaws.com".to_string(),
        zone_id: Some("Z1WCIGYICN2BYD".to_string()),
    });

    m.insert(Region::CaCentral1, RegionInfo {
        title: "Canada (Central)".to_string(),
        s3_endpoint_suffix: "s3-website.ca-central-1.amazonaws.com".to_string(),
        zone_id: Some("Z1QDHH18159H29".to_string()),
    });

    m.insert(Region::EuCentral1, RegionInfo {
        title: "Europe (Frankfurt)".to_string(),
        s3_endpoint_suffix: "s3-website.eu-central-1.amazonaws.com".to_string(),
        zone_id: Some("Z21DNDUVLTQW6Q".to_string()),
    });

    m.insert(Region::EuWest1, RegionInfo {
        title: "Europe (Ireland)".to_string(),
        s3_endpoint_suffix: "s3-website-eu-west-1.amazonaws.com".to_string(),
        zone_id: Some("Z1BKCTXD74EZPE".to_string()),
    });

    m.insert(Region::EuWest2, RegionInfo {
        title: "Europe (London)".to_string(),
        s3_endpoint_suffix: "s3-website.eu-west-2.amazonaws.com".to_string(),
        zone_id: Some("Z3GKZC51ZF0DB4".to_string()),
    });

    m.insert(Region::EuWest3, RegionInfo {
        title: "Europe (Paris)".to_string(),
        s3_endpoint_suffix: "s3-website.eu-west-3.amazonaws.com".to_string(),
        zone_id: Some("Z3R1K369G5AVDG".to_string()),
    });

    m.insert(Region::EuNorth1, RegionInfo {
        title: "Europe (Stockholm)".to_string(),
        s3_endpoint_suffix: "s3-website.eu-north-1.amazonaws.com".to_string(),
        zone_id: Some("Z3BAZG2TWCNX0D".to_string()),
    });

    m.insert(Region::EuSouth1, RegionInfo {
        title: "Europe (Milan)".to_string(),
        s3_endpoint_suffix: "s3-website.eu-south-1.amazonaws.com".to_string(),
        zone_id: None,
    });

    m.insert(Region::MeSouth1, RegionInfo {
        title: "Middle East(Bahrain)".to_string(),
        s3_endpoint_suffix: "s3-website.me-south-1.amazonaws.com".to_string(),
        zone_id: None,
    });

    m.insert(Region::SaEast1, RegionInfo {
        title: "South America (São Paulo)".to_string(),
        s3_endpoint_suffix: "s3-website-sa-east-1.amazonaws.com".to_string(),
        zone_id: Some("Z7KQH4QJS55SO".to_string()),
    });

    m.insert(Region::UsEast1, RegionInfo {
        title: "US East (N. Virginia)".to_string(),
        s3_endpoint_suffix: "s3-website-us-east-1.amazonaws.com".to_string(),
        zone_id: Some("Z3AQBSTGFYJSTF".to_string()),
    });

    m.insert(Region::UsEast2, RegionInfo {
        title: "US East (Ohio)".to_string(),
        s3_endpoint_suffix: "s3-website.us-east-2.amazonaws.com".to_string(),
        zone_id: Some("Z2O1EMRO9K5GLX".to_string()),
    });

    m.insert(Region::UsWest1, RegionInfo {
        title: "US West (N. California)".to_string(),
        s3_endpoint_suffix: "s3-website-us-west-1.amazonaws.com".to_string(),
        zone_id: Some("Z2F56UZL2M1ACD".to_string()),
    });

    m.insert(Region::UsWest2, RegionInfo {
        title: "US West (Oregon)".to_string(),
        s3_endpoint_suffix: "s3-website-us-west-2.amazonaws.com".to_string(),
        zone_id: Some("Z3BJ6K6RIION7M".to_string()),
    });

    m.insert(Region::UsGovEast1, RegionInfo {
        title: "AWS GovCloud (US-East)".to_string(),
        s3_endpoint_suffix: "s3-website.us-gov-east-1.amazonaws.com".to_string(),
        zone_id: Some("Z2NIFVYYW2VKV1".to_string()),
    });

    m.insert(Region::UsGovWest1, RegionInfo {
        title: "AWS GovCloud (US)".to_string(),
        s3_endpoint_suffix: "s3-website-us-gov-west-1.amazonaws.com".to_string(),
        zone_id: Some("Z31GFT0UA1I2HV".to_string()),
    });

    // WARN: An S3 endpoint is not documented for `cn-north-1`
    // WARN: but we follow the convention for `cn-northwest-1` 
    // WARN: so that we don't have to make it Option<String>.
    m.insert(Region::CnNorth1, RegionInfo {
        title: "China (Beijing)".to_string(),
        s3_endpoint_suffix: "s3-website.cn-north-1.amazonaws.com.cn".to_string(),
        zone_id: None,
    });

    m.insert(Region::CnNorthwest1, RegionInfo {
        title: "China (Ningxia)".to_string(),
        s3_endpoint_suffix: "s3-website.cn-northwest-1.amazonaws.com.cn".to_string(),
        zone_id: None,
    });

    m.insert(Region::AfSouth1, RegionInfo {
        title: "Africa (Cape Town)".to_string(),
        s3_endpoint_suffix: "s3-website.af-south-1.amazonaws.com".to_string(),
        zone_id: Some("Z11KHD8FBVPUYU".to_string()),
    });
    m
});

