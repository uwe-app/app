use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use thiserror::Error;

use fluent_templates::ArcLoader;
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

#[derive(Debug)]
pub struct LocaleResult {
    pub multi: bool,
    pub map: BTreeMap<String, LanguageIdentifier>,
}

#[derive(Debug)]
pub struct Locales {
    pub loader: LocalesLoader,
}

impl Default for Locales {
    fn default() -> Self {
        Self {
            //map: BTreeMap::new(),
            loader: Default::default(),
        }
    }
}

impl Locales {

    pub fn get_locale_map(&self, fallback: &str) -> Result<LocaleResult> {
        let mut res = LocaleResult {
            map : BTreeMap::new(),
            multi: self.loader.arc.is_some()
        };

        if let Some(ref arc) = self.loader.arc {
            let langs = arc.locales();
            for lang_id in langs {
                res.map.insert(lang_id.to_string(), lang_id);
            }
        } else {
            let id: LanguageIdentifier = fallback.parse()?;
            res.map.insert(fallback.to_string(), id);
        }
        Ok(res)
    }

    pub fn load(&mut self, config: &Config, options: &RuntimeOptions) -> Result<()> {
        self.loader.load(config, options)?;
        Ok(())
    }
}

pub struct LocalesLoader {
    pub arc: Option<Box<ArcLoader>>,
}

impl Default for LocalesLoader {
    fn default() -> Self {
        Self { arc: None }
    }
}

impl fmt::Debug for LocalesLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").finish()
    }
}

impl LocalesLoader {
    pub fn load(&mut self, config: &Config, options: &RuntimeOptions) -> Result<()> {
        let locales_dir = options.get_locales();
        if locales_dir.exists() && locales_dir.is_dir() {
            let fluent = config.fluent.as_ref().unwrap();
            let result = self.arc(locales_dir, fluent)?;
            self.arc = Some(Box::new(result));
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
            return ArcLoader::builder(dir.as_ref(), fluent.fallback_id.clone())
                .shared_resources(Some(&[core]))
                .build();
        }

        ArcLoader::builder(dir.as_ref(), fluent.fallback_id.clone()).build()
        //.customize(|bundle| bundle.set_use_isolating(false));
    }
}
