use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use url::Url;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct RepositoryConfig {
    #[serde_as(as = "DisplayFromStr")]
    pub url: Url,
    pub edit_path: Option<String>,
}
