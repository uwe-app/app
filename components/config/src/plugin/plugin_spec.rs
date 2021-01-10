use std::fmt;
use std::str::FromStr;

use semver::VersionReq;

use crate::Error;

pub struct PluginSpec {
    pub(crate) name: String,
    pub(crate) range: VersionReq,
}

impl PluginSpec {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn range(&self) -> &VersionReq {
        &self.range
    }
}

impl fmt::Display for PluginSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", &self.name, &self.range)
    }
}

impl fmt::Debug for PluginSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginSpec")
         .field("name", &self.name)
         .field("range", &self.range)
         .finish()
    }
}

impl From<String> for PluginSpec {
    fn from(name: String) -> Self {
        Self {name, range: VersionReq::any()} 
    }
}

impl From<(String, VersionReq)> for PluginSpec {
    fn from(value: (String, VersionReq)) -> Self {
        Self {name: value.0, range: value.1} 
    }
}

impl FromStr for PluginSpec {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.contains(crate::PLUGIN_NS) {
            return Err(Error::InvalidPluginSpecName(s.to_string()))
        }

        let mut name = s.to_string();

        let range = if s.contains(crate::PLUGIN_SPEC) {
            let mut parts = s.splitn(2, crate::PLUGIN_SPEC);
            name = parts.next().unwrap().to_string();
            let version_req = parts.next().unwrap();
            version_req.parse::<VersionReq>()?
        } else { VersionReq::any() };

        Ok(PluginSpec {name, range})
    }
}
