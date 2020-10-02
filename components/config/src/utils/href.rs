use std::fmt;
use std::path::Path;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

/// A marker type for platform agnostic URL paths that should always
/// be delimited by a forward slash. We use this to reference
/// pages which are resolved relative to the site root.

//pub type UrlPath = String;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UrlPath {
    value: String,
}

impl UrlPath {
    pub fn trim_start_matches(&self, val: &str) -> &str {
        self.value.trim_start_matches(val)
    }

    pub fn starts_with(&self, val: &str) -> bool {
        self.value.starts_with(val)
    }
}

impl fmt::Display for UrlPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.value)
    }
}

impl AsRef<str> for UrlPath {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl From<String> for UrlPath {
    fn from(s: String) -> Self {
        Self { value: s }
    }
}

impl From<&Path> for UrlPath {
    fn from(p: &Path) -> Self {
        Self {
            value: p.to_string_lossy().into_owned(),
        }
    }
}

impl From<&str> for UrlPath {
    fn from(s: &str) -> Self {
        Self {
            value: s.to_owned(),
        }
    }
}

impl Serialize for UrlPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for UrlPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = String;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("`string`")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value.to_owned())
            }
        }

        let value = deserializer.deserialize_string(StringVisitor)?;
        Ok(Self { value })
    }
}
