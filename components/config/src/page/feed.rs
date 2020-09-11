pub use jsonfeed::{Attachment};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FeedEntry {
    pub language: Option<String>,
    pub external_url: Option<String>,
    pub image: Option<String>,
    pub banner_image: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
}

