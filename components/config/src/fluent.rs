use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use unic_langid::LanguageIdentifier;

pub static CORE_FTL: &str = "core.ftl";

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct FluentConfig {
    #[serde_as(as = "Option<DisplayFromStr>")]
    fallback: Option<LanguageIdentifier>,
    shared: String,
}

impl FluentConfig {
    pub(crate) fn prepare(&mut self, lang_id: LanguageIdentifier) {
        if self.fallback.is_none() {
            self.fallback = Some(lang_id);
        }
    }

    pub fn fallback(&self) -> &LanguageIdentifier {
        self.fallback.as_ref().unwrap()
    }

    pub fn shared(&self) -> &str {
        &self.shared
    }
}

impl Default for FluentConfig {
    fn default() -> Self {
        Self {
            fallback: None,
            shared: CORE_FTL.to_string(),
        }
    }
}
