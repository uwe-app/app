use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LiveReload {
    pub notify: Option<bool>,
}

impl Default for LiveReload {
    fn default() -> Self {
        Self { notify: Some(true) }
    }
}
