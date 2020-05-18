use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;

use toml::Value;
use toml::de::{Error as TomlError};
use serde_derive::Deserialize;
use inflector::Inflector;
use handlebars::Handlebars;

/// Manages the data associated with a template.
struct TemplateData;

impl TemplateData {
    pub fn new() -> Self {
        TemplateData{}
    }
}
