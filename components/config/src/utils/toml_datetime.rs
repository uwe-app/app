use chrono::prelude::*;
use serde::{Deserialize, Deserializer};

/// Attribute to convert from TOML date time to chronos UTC variant
pub fn from_toml_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    toml::value::Datetime::deserialize(deserializer).map(|s| {
        let d = s.to_string();
        let dt = if d.contains('T') {
            DateTime::parse_from_rfc3339(&d)
                .ok()
                .map(|s| s.naive_local())
        } else {
            NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                .ok()
                .map(|s| s.and_hms(0, 0, 0))
        };

        if let Some(dt) = dt {
            return Some(DateTime::<Utc>::from_utc(dt, Utc));
        }

        None
    })
}
