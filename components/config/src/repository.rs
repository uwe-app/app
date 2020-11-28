use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use url::Url;

static DEFAULT_REPO: &str = "https://github.com/uwe-app/example/";
//static DEFAULT_EDIT: &str = "edit/master/";

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct RepositoryConfig {
    #[serde_as(as = "DisplayFromStr")]
    pub url: Url,
    pub edit_path: Option<String>,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            url: DEFAULT_REPO.parse().unwrap(),
            edit_path: None,
        }
    }
}
