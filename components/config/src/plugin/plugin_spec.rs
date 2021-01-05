use std::str::FromStr;

use semver::VersionReq;

use crate::Error;

pub struct PluginSpec {
    name: String,
    req: VersionReq,
}

impl From<String> for PluginSpec {
    fn from(name: String) -> Self {
        Self {name, req: VersionReq::any()} 
    }
}

impl From<(String, VersionReq)> for PluginSpec {
    fn from(value: (String, VersionReq)) -> Self {
        Self {name: value.0, req: value.1} 
    }
}

impl FromStr for PluginSpec {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.contains(crate::PLUGIN_NS) {
            return Err(Error::InvalidPluginSpecName(s.to_string()))
        }

        let mut name = s.to_string();

        let req = if s.contains(crate::PLUGIN_SPEC) {
            let mut parts = s.splitn(2, crate::PLUGIN_SPEC);
            name = parts.next().unwrap().to_string();
            let version_req = parts.next().unwrap();
            version_req.parse::<VersionReq>()?
        } else { VersionReq::any() };

        Ok(PluginSpec {name, req})
    }
}
