use std::collections::HashSet;

use serde::{Deserialize, Serialize};

// FIXME: complete this implementation!!!!

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TransformConfig {
    pub html: HashSet<TransformType>,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            html: HashSet::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, Eq, PartialEq)]
pub enum TransformType {
    Heading,
}

