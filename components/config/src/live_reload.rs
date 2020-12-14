use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct LiveReload {
    notify: Option<bool>,
    follow_edits: Option<bool>,
}

impl LiveReload {
    pub fn follow_edits(&self) -> bool {
        self.follow_edits.is_some() && self.follow_edits.unwrap()
    }
}

impl Default for LiveReload {
    fn default() -> Self {
        Self {
            notify: Some(true),
            follow_edits: None,
        }
    }
}
