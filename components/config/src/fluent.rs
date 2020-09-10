use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use unic_langid::LanguageIdentifier;

use crate::LANG;

pub static CORE_FTL: &str = "core.ftl";

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FluentConfig {
    pub shared: Option<String>,
    #[serde(skip)]
    pub fallback: Option<String>,
    #[serde(skip)]
    pub fallback_id: LanguageIdentifier,
}

impl FluentConfig {
    pub(crate) fn prepare(&mut self, lang: &str, lang_id: LanguageIdentifier) {
        self.fallback = Some(lang.to_string());
        self.fallback_id = lang_id;
    }
}

impl Default for FluentConfig {
    fn default() -> Self {
        Self {
            fallback: None,
            shared: Some(String::from(CORE_FTL)),
            fallback_id: String::from(LANG).parse().unwrap(),
        }
    }
}
