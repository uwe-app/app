use url::Url;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Author {
    pub name: String,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub url: Option<Url>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub avatar: Option<Url>,
    pub alias: Option<String>,
}

impl Author {
    pub fn into_json_feed(self) -> jsonfeed::Author {
        let url = if let Some(url) = self.url {
            Some(url.to_string())
        } else {
            None
        };
        let avatar = if let Some(avatar) = self.avatar {
            Some(avatar.to_string())
        } else {
            None
        };
        return jsonfeed::Author {
            name: Some(self.name),
            url: url,
            avatar: avatar,
        };
    }
}
