use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DateConfig {
    pub formats: HashMap<String, String>,
}

impl DateConfig {
    pub(crate) fn prepare(&mut self) {
        let mut datetime_formats = HashMap::new();
        datetime_formats.insert("date-short".to_string(), "%F".to_string());
        datetime_formats
            .insert("date-medium".to_string(), "%a %b %e %Y".to_string());
        datetime_formats
            .insert("date-long".to_string(), "%A %B %e %Y".to_string());

        datetime_formats.insert("time-short".to_string(), "%R".to_string());
        datetime_formats.insert("time-medium".to_string(), "%X".to_string());
        datetime_formats.insert("time-long".to_string(), "%r".to_string());

        datetime_formats
            .insert("datetime-short".to_string(), "%F %R".to_string());
        datetime_formats.insert(
            "datetime-medium".to_string(),
            "%a %b %e %Y %X".to_string(),
        );
        datetime_formats
            .insert("datetime-long".to_string(), "%A %B %e %Y %r".to_string());

        for (k, v) in datetime_formats {
            if !self.formats.contains_key(&k) {
                self.formats.insert(k, v);
            }
        }

        // TODO: validate date time format specifiers
    }
}
