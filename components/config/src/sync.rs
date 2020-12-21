use serde::{Deserialize, Serialize};

static REMOTE: &str = "origin";
static BRANCH: &str = "main";

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(default)]
pub struct SyncConfig {
    pub remote: Option<String>,
    pub branch: Option<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            remote: Some(REMOTE.to_string()),
            branch: Some(BRANCH.to_string()),
        }
    }
}
