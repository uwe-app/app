use url::Url;
use serde::{Serialize, Deserialize};
use serde_with::{serde_as, DisplayFromStr, skip_serializing_none};

static DEFAULT_REPO: &str = "https://github.com/uwe-app/example";

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Repository {
    #[serde_as(as = "DisplayFromStr")]
    pub url: Url,
}

impl Default for Repository {
    fn default() -> Self {
        Self {
            url: DEFAULT_REPO.parse().unwrap(),
        }
    }
}
