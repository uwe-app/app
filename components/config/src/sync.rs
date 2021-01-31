use serde::{Deserialize, Serialize};

static REMOTE: &str = "origin";
static BRANCH: &str = "main";

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(default)]
pub struct SyncConfig {
    remote: String,
    branch: String,
}

impl SyncConfig {
    pub fn remote(&self) -> &str {
        &self.remote
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            remote: REMOTE.to_string(),
            branch: BRANCH.to_string(),
        }
    }
}
