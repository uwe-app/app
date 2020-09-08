use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use thiserror::Error;

use fluent_templates::{static_loader, ArcLoader, Loader};
use unic_langid::LanguageIdentifier;

use config::{Config, FluentConfig, RuntimeOptions};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    LanguageIdentifier(#[from] unic_langid::LanguageIdentifierError),
    // For fluent template loader
    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error>),
}

type Result<T> = std::result::Result<T, Error>;

static_loader! {
    pub static LOCALES = {locales: "./locales", fallback_language: "en"};
}

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
    /// List of languages other than the primary fallback language.
    pub translations: Vec<String>,
}

impl LocaleMap {
    /// Get all locale identifiers.
    pub fn get_locales(&self) -> Vec<&str> {
        self.map.keys().map(|s| s.as_str()).collect()
    }

    /// Get all locale identifiers excluding the fallback.
    pub fn get_translations(&self) -> &Vec<String> {
        &self.translations
    }
}

#[derive(Debug, Default)]
pub struct Locales {
    pub loader: LocalesLoader,
    pub languages: LocaleMap,
}

impl Locales {
    fn get_locale_map(&self, fallback: &str) -> Result<LocaleMap> {
        let mut res = LocaleMap {
            fallback: fallback.to_string(),
            map: HashMap::new(),
            enabled: self.loader.arc.is_some(),
            multi: false,
            translations: vec![],
        };

        if let Some(ref arc) = self.loader.arc {
            let langs = arc.locales();
            for lang_id in langs {
                res.map.insert(lang_id.to_string(), lang_id.clone());
            }
        } else {
            let id: LanguageIdentifier = fallback.parse()?;
            res.map.insert(fallback.to_string(), id);
        }

        res.multi = res.map.len() > 1;

        let translations: Vec<String> = res
            .map
            .keys()
            .filter(|s| s.as_str() != fallback)
            .map(|s| s.to_owned())
            .collect();
        res.translations = translations;

        Ok(res)
    }

    pub fn load(
        &mut self,
        config: &Config,
        options: &RuntimeOptions,
    ) -> Result<&LocaleMap> {
        self.loader.load(config, options)?;
        self.languages = self.get_locale_map(&config.lang)?;
        Ok(&self.languages)
    }
}

#[derive(Default)]
pub struct LocalesLoader {
    pub arc: Option<Box<ArcLoader>>,
}

impl fmt::Debug for LocalesLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").finish()
    }
}

impl LocalesLoader {
    pub fn load(
        &mut self,
        config: &Config,
        options: &RuntimeOptions,
    ) -> Result<()> {
        let locales_dir = options.get_locales();
        if locales_dir.exists() && locales_dir.is_dir() {
            if let Some(ref fluent) = config.fluent {
                let result = self.arc(locales_dir, fluent)?;
                self.arc = Some(Box::new(result));
            }
        }
        Ok(())
    }

    fn arc<'a, P: AsRef<Path>>(
        &mut self,
        dir: P,
        fluent: &FluentConfig,
    ) -> std::result::Result<ArcLoader, Box<dyn std::error::Error>> {
        let file = dir.as_ref();
        if let Some(core_file) = &fluent.shared {
            let mut core = file.to_path_buf();
            core.push(core_file);
            return ArcLoader::builder(
                dir.as_ref(),
                fluent.fallback_id.clone(),
            )
            .shared_resources(Some(&[core]))
            .build();
        }

        ArcLoader::builder(dir.as_ref(), fluent.fallback_id.clone()).build()
        //.customize(|bundle| bundle.set_use_isolating(false));
    }
}
