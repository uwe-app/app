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

    /*
    m.insert(Region::CaCentral1, String::from(""));
    m.insert(Region::EuCentral1, String::from(""));
    m.insert(Region::EuWest1, String::from(""));
    m.insert(Region::EuWest2, String::from(""));
    m.insert(Region::EuWest3, String::from(""));
    m.insert(Region::EuNorth1, String::from(""));
    m.insert(Region::EuSouth1, String::from(""));
    m.insert(Region::MeSouth1, String::from(""));
    m.insert(Region::SaEast1, String::from(""));
    m.insert(Region::UsEast1, String::from(""));
    m.insert(Region::UsEast2, String::from(""));
    m.insert(Region::UsWest1, String::from(""));
    m.insert(Region::UsWest2, String::from(""));
    m.insert(Region::UsGovEast1, String::from(""));
    m.insert(Region::UsGovWest1, String::from(""));
    m.insert(Region::CnNorth1, String::from(""));
    m.insert(Region::CnNorthwest1, String::from(""));
    m.insert(Region::AfSouth1, String::from(""));
    */

    /*
Region Name 	Website Endpoint 	Route 53 Hosted Zone ID

US East (Ohio) 	s3-website.us-east-2.amazonaws.com 	Z2O1EMRO9K5GLX
US East (N. Virginia) 3-website-us-east-1.amazonaws.com Z3AQBSTGFYJSTF 
US West (N. California) s3-website-us-west-1.amazonaws.com Z2F56UZL2M1ACD 
US West (Oregon) s3-website-us-west-2.amazonaws.com Z3BJ6K6RIION7M
Africa (Cape Town) s3-website.af-south-1.amazonaws.com Z11KHD8FBVPUYU
Canada (Central) s3-website.ca-central-1.amazonaws.com Z1QDHH18159H29
China (Ningxia) s3-website.cn-northwest-1.amazonaws.com.cn Not supported
Europe (Frankfurt) s3-website.eu-central-1.amazonaws.com Z21DNDUVLTQW6Q
Europe (Ireland) s3-website-eu-west-1.amazonaws.com Z1BKCTXD74EZPE
Europe (London) s3-website.eu-west-2.amazonaws.com Z3GKZC51ZF0DB4
Europe (Milan) s3-website.eu-south-1.amazonaws.com Not supported
Europe (Paris) s3-website.eu-west-3.amazonaws.com Z3R1K369G5AVDG
Europe (Stockholm) s3-website.eu-north-1.amazonaws.com Z3BAZG2TWCNX0D
Middle East(Bahrain) s3-website.me-south-1.amazonaws.com Not supported
South America (São Paulo) s3-website-sa-east-1.amazonaws.com Z7KQH4QJS55SO
AWS GovCloud (US-East) s3-website.us-gov-east-1.amazonaws.com Z2NIFVYYW2VKV1
AWS GovCloud (US) s3-website-us-gov-west-1.amazonaws.com Z31GFT0UA1I2HV
*/

    m
});

