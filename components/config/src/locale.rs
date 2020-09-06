use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

pub type LocaleName = String;
pub type LocaleIdentifier = HashMap<LocaleName, LanguageIdentifier>;

#[derive(Debug, Clone, Default)]
pub struct LocaleMap {
    /// The fallback language is inherited
    /// from the top-level config `lang`
    pub fallback: LocaleName,
    /// Enabled is active when we are able to load
    /// locale files at runtime.
    pub enabled: bool,
    /// Determine if multiple locales are active.
    pub multi: bool,
    /// Map of all locales to the parsed language identifiers.
    pub map: LocaleIdentifier,
}

impl LocaleMap {
    /// Get all locale identifiers.
    pub fn get_locales(&self) -> Vec<&str> {
        self.map.keys().map(|s| s.as_str()).collect() 
    }

    /// Get all locale identifiers excluding the fallback.
    pub fn get_translations(&self) -> Vec<&str> {
        self.map.keys()
            .filter(|s| s != &&self.fallback)
            .map(|s| s.as_str())
            .collect() 
    }
}
