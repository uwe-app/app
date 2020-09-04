use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

pub type LocaleName = String;

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
    pub map: HashMap<LocaleName, LanguageIdentifier>,
}
